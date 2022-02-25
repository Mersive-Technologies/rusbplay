#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use std::ffi::c_void;
use std::os::raw::c_uchar;
use std::ptr::null_mut;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;
use anyhow::{Context, Error};
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

pub struct Transfer {
    pub buff: Vec<i16>,
    pub xfer: *mut libusb_transfer,
}

impl Transfer {
    fn new(cfg: &Config, mut handle: &mut DeviceHandle<GlobalContext>) -> Result<Transfer, Error> {
        unsafe {
            let mut buff = vec![0i16; cfg.pkt_cnt * cfg.pkt_sz / 2];
            let xfer = alloc_xfer(&cfg, &mut handle, &mut buff).context(anyhow!("Error allocating transfer"))?;
            return Ok(Transfer { buff, xfer });
        }
    }
}

pub struct Config {
    pub vid: u16, // VendorID of USB digital-analog converter
    pub pid: u16, // ProductID of USB digital-analog converter
    pub iface: u8, // Speaker interface (as opposed to mic)
    pub ep: c_uchar, // Endpoint 4 - speaker
    pub set_enabled: u8, // 48khz (see descriptors in readme)
    pub set_disable: u8, // Standard is 0=disabled
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

    rusb_event_loop();

    // Find and open device
    let mut handle = open_dev(&cfg).context(anyhow!("Error opening device"))?;

    // allocate transfer
    let mut xfers = vec![];
    for _ in 0..cfg.buff_cnt {
        xfers.push(Transfer::new(&cfg, &mut handle).context("Error creating transfer")?);
    }

    let (result_tail, result_head): (Sender<TransferResult>, Receiver<TransferResult>) = channel();

    let mut samp_idx = 0;
    for (idx, mut xfer) in xfers.iter_mut().enumerate() {
        submit(&cfg, idx, &mut xfer, &mut samp_idx, &mut handle, &result_tail)?;
    }

    while let Ok(res) = result_head.recv() {
        let xfer = &mut xfers[res.idx];
        submit(&cfg, res.idx, xfer, &mut samp_idx, &mut handle, &result_tail)?;
    }

    Ok(())
}

unsafe fn submit(cfg: &Config, idx: usize, xfer: &mut Transfer,
                 samp_idx: &mut usize,
                 handle: &mut DeviceHandle<GlobalContext>, result_tail: &Sender<TransferResult>
) -> Result<(), Error> {
    fill_buff(&mut xfer.buff, samp_idx);
    for _ in 0..2 {
        let ctx = Box::new(TransferContext {
            idx,
            result_tail: result_tail.clone()
        });
        (*xfer.xfer).user_data = Box::into_raw(ctx) as *mut c_void;
        let res = libusb_submit_transfer(xfer.xfer);
        if res == 0 {
            println!("Transfer submitted idx={} result={}", idx, res);
            return Ok(());
        }
        handle.set_alternate_setting(cfg.iface, cfg.set_disable).context(anyhow!("Error disabling"))?;
        handle.set_alternate_setting(cfg.iface, cfg.set_enabled).context(anyhow!("Error enabling"))?;
    }
    Err(anyhow!("Failed to submit!"))
}

unsafe fn rusb_event_loop() {
    let _ = thread::spawn(move || {
        let timeout = Duration::from_millis(100);
        loop {
            let res = GlobalContext::default().handle_events(Some(timeout));
            if res.is_err() {
                error!("Error processing rusb events: {:?}", res.err());
            }
        }
    });
}

unsafe fn open_dev(cfg: &Config) -> Result<DeviceHandle<GlobalContext>, Error> {
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
    if handle.kernel_driver_active(cfg.iface).context(anyhow!("Error checking kernel"))? {
        handle.detach_kernel_driver(cfg.iface).context("Error detatching kernel")?;
    }
    handle.claim_interface(cfg.iface).context(anyhow!("Error claiming interface"))?;
    Ok(handle)
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
    let _ = ctx.result_tail.send(result);
}
