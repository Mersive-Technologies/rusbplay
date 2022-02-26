mod transfer;
mod submission;
mod util;

#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use crate::transfer::Transfer;
use crate::util::{Config, open_dev, rusb_event_loop};

use futures::future::select_all;
use futures::executor;
use anyhow::{Context, Error};

fn main() -> Result<(), Error> {
    pretty_env_logger::init_timed();

    executor::block_on(run())?;

    Ok(())
}

async fn run() -> Result<(), Error> {
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
        buff_cnt: 3,
    };

    rusb_event_loop();

    // Find and open device
    let mut handle = open_dev(&cfg).context(anyhow!("Error opening device"))?;

    // allocate transfer
    let mut samp_idx = 0;
    let mut xfers = vec![];
    for idx in 0usize..cfg.buff_cnt {
        xfers.push(Transfer::new( idx, &cfg, &mut handle).context("Error creating transfer")?);
    }
    let mut subs: Vec<_> = xfers.iter_mut().map(|xfer| xfer.submit(&mut samp_idx)).collect();
    loop {
        let (res, _, mut compl) = select_all(subs.into_iter()).await;
        let res = res.context("Error selecting!")?;
        compl.push(xfers[res.idx].submit(&mut samp_idx));
        subs = compl;
    }
}
