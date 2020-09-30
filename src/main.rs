mod transfer;

#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use anyhow::Error;
use rusb::{DeviceList, GlobalContext, DeviceHandle, UsbContext};
use crate::transfer::{Transfer, Submission};
use futures::executor;
use futures::future::SelectAll;
use std::thread;
use std::time::Duration;

fn main() {
    pretty_env_logger::init_timed();

    let _ = thread::spawn(move || {
        let timeout = Duration::from_millis(100);
        loop {
            let res = GlobalContext::default().handle_events(Some(timeout));
            if res.is_err() {
                error!("Error processing rusb events: {:?}", res.err());
            }
        }
    });

    let res = run();
    if res.is_err() {
        error!("Error: {}", res.err().unwrap());
    }
}

fn run() -> Result<(), Error> {
    executor::block_on(msg_loop())?;
    Ok(())
}

async fn msg_loop() -> Result<(), Error> {
    let list = DeviceList::new()?;
    info!("Found {} devices", list.len());
    let dev = list.iter().find(|dev| {
        let desc = dev.device_descriptor().unwrap();
        desc.vendor_id() == 0x046d && desc.product_id() == 0x0867
    }).ok_or(anyhow!("Error finding item!"))?;
    info!("dev={:?}", dev);
    let mut handle = dev.open()?;
    let iface = 2;
    if handle.kernel_driver_active(iface)? {
        handle.detach_kernel_driver(iface)?;
    }
    handle.claim_interface(iface)?;
    handle.set_alternate_setting(iface, 1)?;

    let ctx = GlobalContext::default();;
    let ep = 1;
    let pkt_cnt = 6;
    let pkt_sz = 128;
    info!("Creating transfers...");
    let transfers: Vec<_> = (0..2).map(|_| Transfer::new(ctx, &handle, ep, pkt_cnt, pkt_sz).unwrap()).collect();
    info!("Submitting transfers...");
    let mut submissions: Vec<Submission> = transfers.iter().map(|xfer| xfer.submit().unwrap()).collect();
    info!("Polling transfers...");

    loop {
        let (res, idx, mut sub2) = futures::future::select_all(submissions.into_iter()).await;
        info!("Result={:?}", res);
        sub2.push(transfers[idx].submit()?);
        submissions = sub2;
    }

    Ok(())
}

