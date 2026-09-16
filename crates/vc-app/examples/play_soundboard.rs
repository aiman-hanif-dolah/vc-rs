use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use vc_app::{AudioHost, Soundboard};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(
        args.next()
            .context("usage: play_soundboard <wav> <main-output> [monitor]")?,
    );
    let output = args.next().context("main output is required")?;
    let monitor = args.next();
    let board = Soundboard::default();
    board.play(path.clone(), AudioHost::default(), output, monitor)?;
    let started = Instant::now();
    loop {
        let state = board.snapshot();
        let monitor = board.monitor_snapshot();
        if let Some(error) = state.error.or(monitor.error) {
            bail!("{error}");
        }
        if state.path.as_ref() == Some(&path)
            && !state.loading
            && !state.playing
            && !monitor.loading
            && !monitor.playing
        {
            println!("Soundboard clip completed once");
            return Ok(());
        }
        if started.elapsed() > Duration::from_secs(30) {
            bail!("Soundboard did not finish");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}
