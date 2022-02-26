use std::os::raw::c_uchar;
use std::thread;
use std::time::Duration;
use anyhow::{Context, Error};
use rusb::{DeviceHandle, DeviceList, GlobalContext, UsbContext};

pub struct Config {
    pub vid: u16, // VendorID of USB digital-analog converter
    pub pid: u16, // ProductID of USB digital-analog converter
    pub iface: u8, // Speaker interface (as opposed to mic)
    pub ep: c_uchar, // Endpoint 4 - speaker
    pub set_enabled: u8, // 48khz (see descriptors in readme)
    pub set_disable: u8, // Standard is 0=disabled
    pub pkt_sz: usize, // 48000hz * 16bits * 2chan = 192,000byte/sec / 192 = 1ms of audio
    pub pkt_cnt: usize, // Each transfer contains 10ms of audio
    pub buff_cnt: usize, // Number of buffers in ring
}

pub fn rusb_event_loop() {
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

pub fn open_dev(cfg: &Config) -> Result<DeviceHandle<GlobalContext>, Error> {
    let list = DeviceList::new()?;
    info!("Found {} devices", list.len());
    let dev = list.iter().find(|dev| {
        match dev.device_descriptor() {
            Ok(desc) => desc.vendor_id() == cfg.vid && desc.product_id() == cfg.pid,
            _ => false
        }
    }).ok_or(anyhow!("Error finding item!"))?;
    info!("dev={:?}", dev);
    let mut handle = dev.open().context("Error opening device!")?;
    if handle.kernel_driver_active(cfg.iface).context(anyhow!("Error checking kernel"))? {
        handle.detach_kernel_driver(cfg.iface).context("Error detatching kernel")?;
    }
    handle.claim_interface(cfg.iface).context(anyhow!("Error claiming interface"))?;
    handle.set_alternate_setting(cfg.iface, cfg.set_disable).context(anyhow!("Error disabling"))?;
    handle.set_alternate_setting(cfg.iface, cfg.set_enabled).context(anyhow!("Error enabling"))?;
    Ok(handle)
}

pub fn fill_buff(buffer: &mut Vec<i16>, samp_idx: &mut usize) {
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
