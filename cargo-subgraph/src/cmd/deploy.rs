//! Subcommand used for creating a new subgraph.

use crate::{
    api::{cargo, graph},
    linker::Linker,
    manifest::Manifest,
    mappings::Mappings,
};
use anyhow::{Context as _, Result};
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

pub fn run(options: Options) -> Result<()> {
    let client = graph::Client::new(options.graph_node);
    let manifest = Manifest::read(
        &options
            .subgraph_manifest_path
            .map(Result::<_>::Ok)
            .unwrap_or_else(|| {
                Ok(cargo::root()?
                    .parent()
                    .context("Cargo manifest has no parent directory")?
                    .join("subgraph.yaml"))
            })?,
    )?;
    client.deploy(
        &options.subgraph_name,
        manifest.link(Linker::new(options.ipfs_node)?, Mappings::compile()?)?,
    )?;

    Ok(())
}
