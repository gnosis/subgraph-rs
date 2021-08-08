//! Subcommand used for creating a new subgraph.

#![allow(dead_code)]

use anyhow::Result;
use std::path::PathBuf;
use structopt::StructOpt;
use url::Url;

#[derive(StructOpt)]
pub struct Options {
    #[structopt(name = "NAME", help = "Name of the subgraph.")]
    subgraph_name: String,

    #[structopt(long, help = "Path to subgraph.yaml")]
    subgraph_manifest_path: Option<PathBuf>,

    #[structopt(long, help = "URL of the Graph node to deploy to.")]
    graph_node: Url,

    #[structopt(long, help = "URL of the IPFS node to upload to.")]
    ipfs_node: Url,
}

pub fn run(_options: Options) -> Result<()> {
    Ok(())
}
