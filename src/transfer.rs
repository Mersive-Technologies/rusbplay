use libusb1_sys::{libusb_transfer, libusb_alloc_transfer, libusb_free_transfer, libusb_submit_transfer, libusb_iso_packet_descriptor};
use anyhow::Error;
use std::ptr::null_mut;
use rusb::{UsbContext, DeviceHandle};
use libusb1_sys::constants::LIBUSB_TRANSFER_TYPE_ISOCHRONOUS;
use futures::Future;
use futures::task::{Context, Poll, Waker};
use std::pin::Pin;
use libc::{c_void};
use futures::channel::oneshot::{Sender, Receiver, channel};
use std::slice;
use byte_slice_cast::AsSliceOf;

#[derive(Debug, Clone, Copy)]
pub struct IsoPacketDescriptor {
    pub length: u32,
    pub actual_length: u32,
    pub status: i32,
}

impl IsoPacketDescriptor {
    pub fn from_libusb(src: &libusb_iso_packet_descriptor) -> IsoPacketDescriptor {
        IsoPacketDescriptor {
            length: src.length,
            actual_length: src.actual_length,
            status: src.status,
        }
    }
}

pub struct TransferContext {
    result_tail: Sender<TransferResult>,
    waker: Waker,
}

impl TransferContext {
    pub fn new(result_tail: Sender<TransferResult>, waker: Waker) -> TransferContext {
        return TransferContext { result_tail, waker };
    }
}

#[derive(Debug, Clone)]
pub struct TransferResult {
    pub status: i32,
    pub actual_length: i32,
}

pub struct Submission {
    native_transfer: *mut libusb_transfer,
    result: Option<Result<TransferResult, Error>>,
    result_head: Receiver<TransferResult>,
    result_tail: Option<Sender<TransferResult>>,
}

impl Submission {
    pub fn new(native_transfer: *mut libusb_transfer) -> Submission {
        let (result_tail, result_head): (Sender<TransferResult>, Receiver<TransferResult>) = channel();
        Submission {
            native_transfer,
            result: None,
            result_head,
            result_tail: Some(result_tail),
        }
    }
}

impl Future for Submission {
    type Output = Result<TransferResult, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.result_tail.is_some() {
            trace!("Submitting transfer...");
            let ctx = Box::new(TransferContext::new(self.result_tail.take().unwrap(), cx.waker().clone()));
            let res = unsafe {
                (*self.native_transfer).user_data = Box::into_raw(ctx) as *mut c_void;
                libusb_submit_transfer(self.native_transfer)
            };
            if res == 0 {
                trace!("Submitted transfer!");
                Poll::Pending
            } else {
                error!("Submission failed!");
                Poll::Ready(Err(anyhow!("libusb_submit_transfer error: {}", res)))
            }
        } else {
            let res = self.result_head.try_recv();
            if res.is_err() {
                error!("Error getting transfer result: {:?}", &res);
                Poll::Ready(Err(Error::from(res.err().unwrap())))
            } else {
                let res = res.unwrap();
                if res.is_some() {
                    trace!("Got transfer result!");
                    Poll::Ready(Ok(res.unwrap()))
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

pub struct Transfer<T: UsbContext> {
    context: T,
    native_transfer: *mut libusb_transfer,
    pub buffer: Vec<i16>,
}

impl<T: UsbContext> Transfer<T> {
    pub fn new(context: T, handle: &DeviceHandle<T>, ep: u8, pkt_cnt: usize, pkt_sz: usize) -> Result<Transfer<T>, Error> {
        unsafe {
            let sz = pkt_cnt * pkt_sz;
            let mut native_transfer = *&libusb_alloc_transfer(pkt_cnt as i32);
            (*native_transfer).dev_handle = handle.as_raw();
            if native_transfer == null_mut() {
                return Err(anyhow!("libusb_alloc_transfer failed!"));
            }
            let mut buffer = vec![0i16; sz / 2];
            (*native_transfer).endpoint = ep;
            (*native_transfer).transfer_type = LIBUSB_TRANSFER_TYPE_ISOCHRONOUS;
            (*native_transfer).timeout = 0;
            (*native_transfer).num_iso_packets = pkt_cnt as i32;
            (*native_transfer).callback = *&iso_complete_handler;
            (*native_transfer).length = sz as i32;
            (*native_transfer).buffer = buffer.as_mut_ptr() as *mut u8;

            let pkt_descs = (*native_transfer).iso_packet_desc.as_mut_ptr();
            for i in 0..pkt_cnt {
                let pkt_desc = pkt_descs.add(i as usize);
                (*pkt_desc).length = pkt_sz as u32;
                (*pkt_desc).actual_length = 0;
                (*pkt_desc).status = 0;
            }

            let xfer = Transfer { context, native_transfer, buffer };
            Ok(xfer)
        }
    }

    pub fn submit(&self) -> Result<Submission, Error> {
        Ok(Submission::new(self.native_transfer))
    }
}

extern "system" fn iso_complete_handler(xfer: *mut libusb_transfer) {
    let ctx = unsafe {
        Box::from_raw((*xfer).user_data as *mut TransferContext)
    };
    let xfer = unsafe { &*xfer };
    trace!("Transfer completed with status: {}", xfer.status);
    let result = TransferResult {
        status: xfer.status,
        actual_length: xfer.actual_length,
    };
    ctx.result_tail.send(result);
    ctx.waker.wake();
}

impl<T: UsbContext> Drop for Transfer<T> {
    fn drop(&mut self) {
        unsafe {
            libusb_free_transfer(self.native_transfer);
        }
    }
}
