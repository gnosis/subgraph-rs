//! Module for `cargo-subgraph` subcommands.

use anyhow::Result;
use structopt::StructOpt;

pub mod create;
pub mod deploy;

#[derive(StructOpt)]
#[structopt(name = "cargo-subgraph", about = "Manage subgraphs written in Rust 🦀")]
pub enum Options {
    #[structopt(about = "Create a new subgraph.")]
    Create(create::Options),
    #[structopt(about = "Build and deploy a subgraph.")]
    Deploy(deploy::Options),
}

pub fn run() -> Result<()> {
    match Options::from_args() {
        Options::Create(options) => create::run(options),
        Options::Deploy(options) => deploy::run(options),
    }
}
