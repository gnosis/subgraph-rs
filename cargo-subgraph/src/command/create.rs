//! Subgraph name registration subcommand implementation.

use crate::api::graph;
use anyhow::Result;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(about = "Register a subgraph name with a Graph node.")]
pub struct Options {
    /// Graph access token.
    #[structopt(long, env = "GRAPH_ACCESS_TOKEN")]
    access_token: Option<String>,

    /// Graph node to create the subgraph in.
    #[structopt(short = "g", long)]
    node: String,
}

/// Run the `create` subcommand.
pub fn run(options: Options) -> Result<()> {
    let graph = graph::Client::new(options.node, options.access_token);
    graph.subgraph_create("gnosis/deep-thought")?;

    Ok(())
}
