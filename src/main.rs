#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use anyhow::{Context, Error};
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::SinkExt;
use libusb1_sys::constants::LIBUSB_TRANSFER_TYPE_ISOCHRONOUS;
use libusb1_sys::{libusb_alloc_transfer, libusb_submit_transfer, libusb_transfer};
use rusb::{DeviceList, GlobalContext, UsbContext};

#[derive(Debug, Clone)]
pub struct TransferResult {
    pub status: i32,
    pub actual_length: i32,
}

pub struct TransferContext {
    done: Arc<AtomicBool>,
}

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
    handle.reset().unwrap();
    handle.unconfigure();
    let res = handle.set_active_configuration(1);
    info!("set config res={:?}", res);
    if handle.kernel_driver_active(2).unwrap() { handle.detach_kernel_driver(2).unwrap(); }
    handle.claim_interface(2).unwrap();
    handle.set_alternate_setting(2, 0).unwrap();
    handle.set_alternate_setting(2, 2).unwrap();

    // do math
    let ep = 4;
    let pkt_cnt = 10;
    let pkt_sz = 192;
    let sz = pkt_cnt * pkt_sz; // One transfer can have many packets
    let mut buffer = vec![0i16; sz / 2];

    // allocate transfer
    let mut native_transfer = *&libusb_alloc_transfer(pkt_cnt as i32);
    if native_transfer == null_mut() {
        return Err(anyhow!("libusb_alloc_transfer failed!"));
    }
    (*native_transfer).dev_handle = handle.as_raw();
    (*native_transfer).endpoint = ep;
    (*native_transfer).transfer_type = LIBUSB_TRANSFER_TYPE_ISOCHRONOUS;
    (*native_transfer).timeout = 0;
    (*native_transfer).num_iso_packets = pkt_cnt as i32;
    (*native_transfer).callback = *&iso_complete_handler;
    (*native_transfer).length = sz as i32;
    (*native_transfer).buffer = buffer.as_mut_ptr() as *mut u8;

    // Fill in packet descriptors
    let pkt_descs = (*native_transfer).iso_packet_desc.as_mut_ptr();
    for i in 0..pkt_cnt {
        let pkt_desc = pkt_descs.add(i as usize);
        (*pkt_desc).length = pkt_sz as u32;
        (*pkt_desc).actual_length = 0;
        (*pkt_desc).status = 0;
    }

    let (result_tail, result_head): (Sender<TransferResult>, Receiver<TransferResult>) = channel(0);
    let mut done = Arc::new(AtomicBool::new(false));

    let volume = 0.05f32;
    let tone_hz = 440f32;
    let samp_per_sec = 48000f32;
    let ang_per_samp = std::f32::consts::PI * 2f32 / samp_per_sec * tone_hz;
    let mut samp_idx = 0;
    loop {
        for buff_idx in 0..buffer.len() {
            let abs_samp = (samp_idx + buff_idx) as f32;
            let phase = (abs_samp * ang_per_samp).sin();
            let volume = phase * volume;
            let scaled = volume * std::i16::MAX as f32;
            buffer[buff_idx] = scaled as i16;
        }
        samp_idx += buffer.len();

        done.store(false, Ordering::Relaxed);
        let ctx = Box::new(TransferContext { done: done.clone() });
        (*native_transfer).user_data = Box::into_raw(ctx) as *mut c_void;
        let res = libusb_submit_transfer(native_transfer);
        if res != 0 {
            handle.set_alternate_setting(2, 0).unwrap();
            handle.set_alternate_setting(2, 2).unwrap();

            let ctx = Box::new(TransferContext { done: done.clone() });
            (*native_transfer).user_data = Box::into_raw(ctx) as *mut c_void;
            let res = libusb_submit_transfer(native_transfer);
        }
        println!("Transfer submitted {}", res);
        let timeout = Duration::from_millis(100);
        loop {
            println!("Handling events");
            GlobalContext::default().handle_events(Some(timeout)).context("Error handling events!")?;
            if done.load(Ordering::Relaxed) {
                break;
            }
        }
        println!("Handled events");
    }

    Ok(())
}

extern "system" fn iso_complete_handler(xfer: *mut libusb_transfer) {
    println!("Transfer complete!");
    let mut ctx = unsafe {
        Box::from_raw((*xfer).user_data as *mut TransferContext)
    };
    let xfer = unsafe { &*xfer };
    trace!("Transfer completed with status: {}", xfer.status);
    let result = TransferResult {
        status: xfer.status,
        actual_length: xfer.actual_length,
    };
    ctx.done.store(true, Ordering::Relaxed);
}
