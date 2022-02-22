#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use std::ptr::null_mut;
use anyhow::{Context, Error};
use libusb1_sys::constants::LIBUSB_TRANSFER_TYPE_ISOCHRONOUS;
use libusb1_sys::{libusb_alloc_transfer, libusb_transfer};
use rusb::{DeviceList};

fn main() -> Result<(), Error> {
    println!("Hello, world!");

    unsafe {
        run();
    }

    println!("Done!");
    Ok(())
}

unsafe fn run() -> Result<(), Error> {
    let list = DeviceList::new()?;
    info!("Found {} devices", list.len());
    let dev = list.iter().find(|dev| {
        match dev.device_descriptor() {
            Ok(desc) => {
                desc.vendor_id() == 0x0bda && desc.product_id() == 0x48a8
            },
            _ => false
        }
    }).ok_or(anyhow!("Error finding item!"))?;
    println!("dev={:?}", dev);

    let mut handle = dev.open().context("Error opening device!")?;

    let ep = 4;
    let pkt_cnt = 10;
    let pkt_sz = 192;

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

    Ok(())
}

extern "system" fn iso_complete_handler(_xfer: *mut libusb_transfer) {
    println!("Transfer complete!");
}
