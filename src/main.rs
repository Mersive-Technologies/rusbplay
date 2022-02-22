#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use anyhow::{Error};
use rusb::{DeviceList};

fn main() -> Result<(), Error> {
    println!("Hello, world!");

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

    Ok(())
}

