#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use std::ffi::c_void;
use std::os::raw::c_uchar;
use std::ptr::null_mut;
use std::time::Duration;
use anyhow::{Context, Error};
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::SinkExt;
use libusb1_sys::constants::LIBUSB_TRANSFER_TYPE_ISOCHRONOUS;
use libusb1_sys::{libusb_alloc_transfer, libusb_submit_transfer, libusb_transfer};
use rusb::{DeviceHandle, DeviceList, GlobalContext, UsbContext};

#[derive(Debug, Clone)]
pub struct TransferResult {
    pub idx: usize,
    pub status: i32,
    pub actual_length: i32,
}

pub struct TransferContext {
    pub idx: usize,
    result_tail: Sender<TransferResult>,
}

pub struct Config {
    pub vid: u16, // VendorID of USB digital-analog converter
    pub pid: u16, // ProductID of USB digital-analog converter
    pub iface: u8, // Speaker interface (as opposed to mic)
    pub ep: c_uchar, // Endpoint 4 - speaker
    pub set_enabled: u8, // 48khz (see descriptors in readme)
    pub set_disable: u8, // Standard
    pub pkt_sz: usize, // 48000hz * 16bits * 2chan = 192,000byte/sec / 192 = 1ms of audio
    pub pkt_cnt: usize, // Each transfer contains 10ms of audio
    pub buff_cnt: i32, // Number of buffers in ring
}

fn main() -> Result<(), Error> {
    println!("Hello, world!");

    unsafe {
        run()?;
    }

    println!("Done!");
    Ok(())
}

unsafe fn run() -> Result<(), Error> {
    // do math
    let cfg = Config {
        vid: 0x0bda,
        pid: 0x48a8,
        iface: 2,
        ep: 4,
        set_enabled: 2,
        set_disable: 0,
        pkt_sz: 192,
        pkt_cnt: 10,
        buff_cnt: 2,
    };

    // Find and open device
    let list = DeviceList::new()?;
    info!("Found {} devices", list.len());
    let dev = list.iter().find(|dev| {
        match dev.device_descriptor() {
            Ok(desc) => desc.vendor_id() == cfg.vid && desc.product_id() == cfg.pid,
            _ => false
        }
    }).ok_or(anyhow!("Error finding item!"))?;
    println!("dev={:?}", dev);
    let mut handle = dev.open().context("Error opening device!")?;
    handle.claim_interface(cfg.iface).unwrap();
    
    // allocate transfer
    let mut buffers: Vec<Vec<i16>> = (0..cfg.buff_cnt).map(|_| vec![0i16; cfg.pkt_cnt * cfg.pkt_sz / 2]).collect();
    let (xfers, errors): (Vec<_>, Vec<_>) = buffers.iter().map(|mut b|
        alloc_xfer(&cfg, &mut handle, &mut b)
    ).partition(Result::is_ok);
    if let Some(e) = errors.first() {
        e.context("Error allocating transfer")?;
    }
    let xfers: Vec<_> = xfers.into_iter().map(Result::unwrap).collect();

    let (result_tail, result_head): (Sender<TransferResult>, Receiver<TransferResult>) = channel(0);

    let mut samp_idx = 0;
    for (idx, xfer) in xfers.iter().enumerate() {
        let mut buffer = &mut buffers[idx];
        fill_buff(buffer, &mut samp_idx);
        submit(&cfg, idx, &mut handle, &result_tail)?;
    }

    loop {

        submit(&cfg, &mut handle, &result_tail);

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
}

unsafe fn submit(cfg: &Config, idx: usize,
                 handle: &mut DeviceHandle<GlobalContext>, result_tail: &Sender<TransferResult>
) -> Result<(), Error> {
    for _ in 0..2 {
        let ctx = Box::new(TransferContext {
            idx,
            result_tail: result_tail.clone()
        });
        (*xfer).user_data = Box::into_raw(ctx) as *mut c_void;
        let res = libusb_submit_transfer(xfer);
        if res == 0 {
            println!("Transfer submitted {}", res);
            break;
        }
        handle.set_alternate_setting(cfg.iface, cfg.set_disable).unwrap();
        handle.set_alternate_setting(cfg.iface, cfg.set_enabled).unwrap();
    }
    Err(anyhow!("Failed to submit!"))
}

unsafe fn fill_buff(buffer: &mut Vec<i16>, samp_idx: &mut usize) {
    let volume = 0.05f32; // 5% to save my ears
    let tone_hz = 440f32; // pitch standard "A" note
    let samp_per_sec = 48000f32; // frequency of alt setting #2 (see descriptors in readme)
    let ang_per_samp = std::f32::consts::PI * 2f32 / samp_per_sec * tone_hz;
    for buff_idx in 0..buffer.len() {
        let abs_samp = (*samp_idx + buff_idx) as f32;
        let phase = (abs_samp * ang_per_samp).sin();
        let volume = phase * volume;
        let scaled = volume * std::i16::MAX as f32;
        buffer[buff_idx] = scaled as i16;
    }
    (*samp_idx) += buffer.len();
}

unsafe fn alloc_xfer(cfg: &Config,
                     handle: &mut DeviceHandle<GlobalContext>,
                     buffer: &mut Vec<i16>
) -> Result<*mut libusb_transfer, Error> {
    let sz = cfg.pkt_cnt * cfg.pkt_sz;
    let mut xfer = *&libusb_alloc_transfer(cfg.pkt_cnt as i32);
    if xfer == null_mut() {
        Err(anyhow!("libusb_alloc_transfer failed!"))?;
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
    println!("Transfer complete!");
    let ctx = unsafe {
        Box::from_raw((*xfer).user_data as *mut TransferContext)
    };
    let xfer = unsafe { &*xfer };
    trace!("Transfer completed with status: {}", xfer.status);
    let result = TransferResult {
        idx: ctx.idx,
        status: xfer.status,
        actual_length: xfer.actual_length,
    };
    ctx.result_tail.send(result);
}
