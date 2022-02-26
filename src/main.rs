mod transfer;
mod submission;
mod util;

#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use crate::transfer::Transfer;
use crate::util::{Config, fill_buff, open_dev, rusb_event_loop};

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
    let mut submissions = vec![];
    for idx in 0usize..cfg.buff_cnt {
        xfers.push(Transfer::new( idx, &cfg, &mut handle).context("Error creating transfer")?);
    }
    for xfer in &mut xfers {
        fill_buff(&mut xfer.buff, &mut samp_idx);
        let submission = xfer.submit();
        submissions.push(submission);
    }
    loop {
        let (res, _, mut remaining) = futures::future::select_all(submissions.into_iter()).await;
        let res = res.context("Error selecting!")?;
        let xfer = &mut xfers[res.idx];
        info!("Transfer {}/{} complete", xfer.idx, res.idx);

        fill_buff(&mut xfer.buff, &mut samp_idx);
        let submission = xfer.submit();
        remaining.push(submission);

        submissions = remaining;
    }
}
