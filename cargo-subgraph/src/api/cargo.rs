//! A simple API wrapper around the `cargo` binary.

use anyhow::{ensure, Result};
use serde::Deserialize;
use serde_json::{Deserializer, Value};
use std::{env, ffi::OsStr, path::PathBuf, process::Command};

/// Retrieves the root of the crate in the current working directory.
/// directory.
pub fn root() -> Result<PathBuf> {
    let output = cargo(|command| command.arg("locate-project"))?;
    let project = serde_json::from_slice::<ProjectLocation>(&output)?;

    Ok(project.root)
}

/// Returns the target directory for the crate in the current working directory.
pub fn target_directory() -> Result<PathBuf> {
    let output = cargo(|command| command.args(&["metadata", "--format-version", "1"]))?;
    let metadata = serde_json::from_slice::<Metadata>(&output)?;

    Ok(metadata.target_directory)
}

/// Builds a project as a Wasm module.
///
/// Returns all Wasm module compiler artifacts.
pub fn build_wasm() -> Result<Vec<PathBuf>> {
    let output = cargo(|command| {
        command.args(&[
            "build",
            "--lib",
            "--release",
            "--target=wasm32-unknown-unknown",
            "--message-format=json",
        ])
    })?;
    let modules = Deserializer::from_slice(&output)
        .into_iter::<Message>()
        .try_fold(Vec::new(), |mut modules, message| -> Result<_> {
            let message = message?;
            if message.reason == "compiler-artifact" {
                let artifact = serde_json::from_value::<CompilerArtifact>(message.data)?;
                modules.extend(
                    artifact
                        .filenames
                        .into_iter()
                        .filter(|filename| filename.extension() == Some(OsStr::new("wasm"))),
                );
            }
            Ok(modules)
        })?;

    Ok(modules)
}

#[derive(Deserialize)]
struct ProjectLocation {
    root: PathBuf,
}

#[derive(Deserialize)]
struct Metadata {
    target_directory: PathBuf,
}

#[derive(Deserialize)]
struct Message {
    reason: String,
    #[serde(flatten)]
    data: Value,
}

#[derive(Deserialize)]
struct CompilerArtifact {
    filenames: Vec<PathBuf>,
}

#[derive(Deserialize)]
struct Resolve {
    #[serde(default)]
    root: String,
}

fn cargo(config: impl FnOnce(&mut Command) -> &mut Command) -> Result<Vec<u8>> {
    let mut cargo = Command::new(env::var("CARGO").as_deref().unwrap_or("cargo"));
    let output = config(&mut cargo).output()?;
    ensure!(
        output.status.success(),
        "error running `{:?}`: {}",
        cargo,
        String::from_utf8_lossy(&output.stderr),
    );

    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn for_each_sample<T>(f: impl Fn() -> Result<T>, p: impl Fn(T)) {
        struct Dir(PathBuf);
        impl Drop for Dir {
            fn drop(&mut self) {
                env::set_current_dir(&self.0).unwrap();
            }
        }

        let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../samples");
        for sample in samples.read_dir().unwrap() {
            let output = {
                let _dir = Dir(env::current_dir().unwrap());
                env::set_current_dir(&sample.unwrap().path()).unwrap();
                f()
            };
            p(output.unwrap());
        }
    }

    #[test]
    fn sample_paths() {
        for_each_sample(
            || Ok((root()?, target_directory()?)),
            |(root, target)| {
                println!("Found sample root '{}'", root.display());
                println!("           target '{}'", target.display());
            },
        );
    }

    #[test]
    #[ignore]
    fn sample_builds() {
        println!("Sample Wasm build artifacts:");
        for_each_sample(
            build_wasm,
            |modules| {
                for module in modules {
                    println!(" - '{}'", module.display());
                }
            },
        );
    }
}
