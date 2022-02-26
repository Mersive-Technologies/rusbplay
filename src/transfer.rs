use crate::util::{Config, fill_buff};
use crate::submission::Submission;

use futures::task::{Waker};
use std::ptr::null_mut;
use futures::channel::oneshot::{Sender};
use anyhow::{Context, Error};
use libusb1_sys::constants::LIBUSB_TRANSFER_TYPE_ISOCHRONOUS;
use libusb1_sys::{libusb_alloc_transfer, libusb_transfer};
use rusb::{DeviceHandle, GlobalContext};

#[derive(Debug, Clone)]
pub struct TransferResult {
    pub idx: usize,
    pub status: i32,
    pub actual_length: i32,
}

pub struct TransferContext {
    pub idx: usize,
    pub result_tail: Sender<TransferResult>,
    pub waker: Waker,
}

pub struct Transfer {
    pub idx: usize,
    pub buff: Vec<i16>,
    pub xfer: *mut libusb_transfer,
}

impl Transfer {
    pub fn new(idx: usize, cfg: &Config, mut handle: &mut DeviceHandle<GlobalContext>) -> Result<Transfer, Error> {
        unsafe {
            let mut buff = vec![0i16; cfg.pkt_cnt * cfg.pkt_sz / 2];
            let xfer = alloc_xfer(&cfg, &mut handle, &mut buff).context(anyhow!("Error allocating transfer"))?;
            return Ok(Transfer { idx, buff, xfer });
        }
    }

    pub fn submit(&mut self, samp_idx: &mut usize) -> Submission {
        fill_buff(&mut self.buff, samp_idx);
        Submission::new(self)
    }
}

unsafe fn alloc_xfer(cfg: &Config,
                     handle: &mut DeviceHandle<GlobalContext>,
                     buffer: &mut Vec<i16>
) -> Result<*mut libusb_transfer, Error> {
    let sz = cfg.pkt_cnt * cfg.pkt_sz;
    let mut xfer = *&libusb_alloc_transfer(cfg.pkt_cnt as i32);
    if xfer == null_mut() {
        return Err(anyhow!("libusb_alloc_transfer failed!"));
    }
    (*xfer).dev_handle = handle.as_raw();
    (*xfer).endpoint = cfg.ep;
    (*xfer).transfer_type = LIBUSB_TRANSFER_TYPE_ISOCHRONOUS; // reserve seats on the bus
    (*xfer).timeout = 0;
    (*xfer).num_iso_packets = cfg.pkt_cnt as i32;
    (*xfer).callback = *&iso_complete_handler;
    (*xfer).length = sz as i32;
    (*xfer).buffer = buffer.as_mut_ptr() as *mut u8;

    // Fill in packet descriptors
    let pkt_descs = (*xfer).iso_packet_desc.as_mut_ptr();
    for i in 0usize..cfg.pkt_cnt {
        let pkt_desc = pkt_descs.add(i);
        (*pkt_desc).length = cfg.pkt_sz as u32;
        (*pkt_desc).actual_length = 0;
        (*pkt_desc).status = 0;
    }
    Ok(xfer)
}

extern "system" fn iso_complete_handler(xfer: *mut libusb_transfer) {
    let ctx = unsafe {
        Box::from_raw((*xfer).user_data as *mut TransferContext)
    };
    let xfer = unsafe { &*xfer };
    info!("ISO transfer {} complete!", ctx.idx);
    let result = TransferResult {
        idx: ctx.idx,
        status: xfer.status,
        actual_length: xfer.actual_length,
    };
    let _ = ctx.result_tail.send(result);
    ctx.waker.wake();
}

