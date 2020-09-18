mod api;
mod command;

use self::command::*;
use anyhow::Result;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(about)]
enum Options {
    Build(build::Options),
    Create(create::Options),
    Deploy(deploy::Options),
}

fn main() -> Result<()> {
    match Options::from_args() {
        Options::Build(options) => build::run(options),
        Options::Create(options) => create::run(options),
        Options::Deploy(options) => deploy::run(options),
    }
}
