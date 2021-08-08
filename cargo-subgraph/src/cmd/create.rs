//! Subcommand used for creating a new subgraph.

use crate::api::graph;
use anyhow::{Context as _, Result};
use structopt::StructOpt;
use url::Url;

#[derive(StructOpt)]
pub struct Options {
    #[structopt(name = "NAME", help = "Name of the subgraph.")]
    subgraph_name: String,

    #[structopt(long, help = "URL of the Graph node to create the subgraph on.")]
    graph_node: Url,
}

pub fn run(options: Options) -> Result<()> {
    let client = graph::Client::new(options.graph_node);
    client
        .create(&options.subgraph_name)
        .context("Error creating subgraph")?;

    Ok(())
}
