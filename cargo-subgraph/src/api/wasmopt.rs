//! A simple API wrapper around the `wasm-opt` binary.

use anyhow::{ensure, Result};
use std::{env, path::Path, process::Command};

/// Optimizes a module in place.
pub fn optimize_in_place(file: &Path, opt_level: &str) -> Result<()> {
    let mut command = Command::new(env::var("WASM_OPT").as_deref().unwrap_or("wasm-opt"));
    let output = command
        .args(&[
            format!("-O{}", opt_level).as_ref(),
            "--strip-debug".as_ref(),
            file.as_os_str(),
            "-o".as_ref(),
            file.as_os_str(),
        ])
        .output()?;
    ensure!(
        output.status.success(),
        "error running `{:?}`: {}",
        command,
        String::from_utf8_lossy(&output.stderr),
    );

    Ok(())
}
