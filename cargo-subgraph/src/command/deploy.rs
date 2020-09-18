//! Subgraph deployment subcommand implementation.

use crate::api::{graph, ipfs};
use anyhow::Result;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(about = "Deploy a subgraph to a Graph node.")]
pub struct Options {
    /// Graph access token.
    #[structopt(long, env = "GRAPH_ACCESS_TOKEN")]
    access_token: Option<String>,

    /// IPFS node to upload build results to.
    #[structopt(short, long)]
    ipfs: String,

    /// Graph node to create the subgraph in.
    #[structopt(short = "g", long)]
    node: String,

    /// Output directory for build output. If this option is not specified
    /// then the build output will be directly uploaded to IPFS without
    /// being stored on the local file system.
    #[structopt(short, long)]
    output_dir: Option<PathBuf>,
}

/// Run the `deploy` subcommand.
pub fn run(options: Options) -> Result<()> {
    let ipfs = ipfs::Client::new(options.ipfs);
    let main = ipfs.add("src/main.rs", b"Hello IPFS!"[..].to_owned())?;
    ipfs.pin(&main.hash)?;

    let graph = graph::Client::new(options.node, options.access_token);
    graph.subgraph_deploy("gnosis/deep-thought", &main.hash)?;

    Ok(())
}
