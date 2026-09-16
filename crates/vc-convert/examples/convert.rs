use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use vc_convert::{convert_pth_file, ConvertOptions};

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let source = PathBuf::from(args.next().context("usage: convert <model.pth>")?);
    if args.next().is_some() {
        bail!("usage: convert <model.pth>");
    }
    if source.with_extension("onnx").exists() {
        bail!("output already exists; choose a different source basename");
    }
    let output = convert_pth_file(&source, &ConvertOptions::default(), &mut |stage| {
        eprintln!("{}", stage.label());
    })?;
    println!("{}", output.display());
    Ok(())
}
