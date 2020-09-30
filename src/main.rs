mod transfer;

#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use anyhow::Error;
use rusb::{DeviceList, GlobalContext, DeviceHandle, UsbContext};
use crate::transfer::Transfer;
use futures::executor;
use futures::future::SelectAll;

fn main() {
    pretty_env_logger::init_timed();
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
    let transfers = (0..1).map(|_| Transfer::new(ctx, &handle, ep, pkt_cnt, pkt_sz).unwrap());
    let submissions: Vec<_> = transfers.map(|xfer| xfer.submit().unwrap()).collect();

    let f3: SelectAll<_> = futures::future::select_all(submissions.into_iter());

    Ok(())
}

