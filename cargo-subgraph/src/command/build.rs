//! Subgraph build subcommand implementation.

use anyhow::Result;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(about = "Build a Rust subgraph.")]
pub struct Options {
    /// Output directory for build files. If this option is not specified
    /// then no output files will be written.
    #[structopt(short, long)]
    output_dir: Option<PathBuf>,
}

/// Run the `build` subcommand.
pub fn run(options: Options) -> Result<()> {
    Ok(())
}
