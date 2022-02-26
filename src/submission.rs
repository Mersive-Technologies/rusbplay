use std::ffi::c_void;
use futures::Future;
use futures::task::{Poll};
use std::pin::Pin;
use futures::channel::oneshot::{Sender, Receiver, channel};
use anyhow::{Error};
use libusb1_sys::{libusb_submit_transfer, libusb_transfer};
use crate::transfer::{Transfer, TransferContext, TransferResult};

pub struct Submission {
    xfer: *mut libusb_transfer,
    result_head: Receiver<TransferResult>,
    result_tail: Option<Sender<TransferResult>>,
    idx: usize,
}

impl Submission {
    pub fn new(xfer: &Transfer) -> Submission {
        let (result_tail, result_head): (Sender<TransferResult>, Receiver<TransferResult>) = channel();
        Submission {
            xfer: xfer.xfer,
            result_head,
            result_tail: Some(result_tail),
            idx: xfer.idx,
        }
    }
}

impl Drop for Submission {
    fn drop(&mut self) {
        info!("Dropped submission {}", self.idx);
    }
}

impl Future for Submission {
    type Output = Result<TransferResult, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut futures::task::Context<'_>) -> Poll<Self::Output> {
        if let Some(result_tail) = self.result_tail.take() {
            let ctx = Box::new(TransferContext {
                waker: cx.waker().clone(),
                idx: self.idx,
                result_tail,
            });
            unsafe {
                (*self.xfer).user_data = Box::into_raw(ctx) as *mut c_void;
                let res = libusb_submit_transfer(self.xfer);
                if res == 0 {
                    info!("Transfer submitted idx={} result={}", self.idx, res);
                    Poll::Pending
                } else {
                    error!("Submission failed!");
                    Poll::Ready(Err(anyhow!("libusb_submit_transfer error: {}", res)))
                }
            }
        } else {
            let res = self.result_head.try_recv();
            match res {
                Err(e) => {
                    error!("Error getting transfer result: {:?}", &res);
                    Poll::Ready(Err(Error::from(e)))
                },
                Ok(Some(res)) => Poll::Ready(Ok(res)),
                _ => Poll::Pending
            }
        }
    }
}
