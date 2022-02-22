#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use anyhow::Error;
use rusb::{DeviceList, GlobalContext, DeviceHandle, UsbContext};
use futures::executor;
use futures::future::SelectAll;
use std::thread;
use std::time::Duration;

fn main() {
    println!("Hello, world!");
}

