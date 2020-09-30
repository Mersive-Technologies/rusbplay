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
    let mut transfers: Vec<_> = (0..3).map(|_| Transfer::new(ctx, &handle, ep, pkt_cnt, pkt_sz).unwrap()).collect();
    info!("Submitting transfers...");
    let mut submissions: Vec<Submission> = transfers.iter().map(|xfer| xfer.submit().unwrap()).collect();
    info!("Polling transfers...");

    let volume = 0.8f32;
    let tone_hz = 440f32;
    let samp_per_sec = 32000f32;
    let ang_per_samp = std::f32::consts::PI * 2f32 / samp_per_sec * tone_hz;
    let mut samp_idx = 0;
    loop {
        let (res, idx, mut sub2) = futures::future::select_all(submissions.into_iter()).await;
        trace!("Result={:?}", res);
        if res.is_err() {
            error!("Error transferring: {:?}", res);
        } else {
            let status = res.unwrap().status;
            if status != 0 {
                error!("Error transferring: {:?}", status);
            }
        }
        let mut xfer = &mut transfers[idx];

        for buff_idx in 0..xfer.buffer.len() {
            let abs_samp = (samp_idx + buff_idx) as f32;
            let phase = (abs_samp * ang_per_samp).sin();
            let volume = phase * volume;
            let scaled = volume * std::i16::MAX as f32;
            xfer.buffer[buff_idx] = scaled as i16;
        }
        samp_idx += xfer.buffer.len();

        sub2.push(xfer.submit()?);
        submissions = sub2;
    }

    Ok(())
}

