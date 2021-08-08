//! A simple API wrapper around the `cargo` binary.

use anyhow::{ensure, Context as _, Result};
use serde::Deserialize;
use std::{env, path::PathBuf, process::Command};

/// Module metadata for WASM target crate.
pub struct Module {
    pub root: PathBuf,
    pub path: PathBuf,
}

/// Retrieves the WASM module metadata for the crate at the current working
/// directory.
pub fn module() -> Result<Module> {
    let output = cargo()
        .args(&["metadata", "--format-version", "1"])
        .output()?;
    ensure!(
        output.status.success(),
        "error running `cargo metadata --format-version 1`: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let metadata = serde_json::from_slice::<Metadata>(&output.stdout)?;
    let package = metadata
        .packages
        .iter()
        .find(|package| package.id == metadata.resolve.root)
        .context("crate is virtual")?;
    let target = package
        .targets
        .iter()
        .find(|target| target.kind.iter().any(|kind| kind == "cdylib"))
        .with_context(|| {
            format!(
                "'{}' is not configured for generating a Wasm module. \
                 Make sure `lib` target kind includes `cdylib`.",
                package.name,
            )
        })?;

    Ok(Module {
        root: package
            .manifest_path
            .parent()
            .context("crate manifest has no root")?
            .to_owned(),
        path: metadata.target_directory.join(format!(
            "wasm32-unknown-unknown/release/{}.wasm",
            target.name.replace('-', "_"),
        )),
    })
}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    resolve: Resolve,
    target_directory: PathBuf,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    id: String,
    targets: Vec<Target>,
    manifest_path: PathBuf,
}

#[derive(Deserialize)]
struct Target {
    kind: Vec<String>,
    name: String,
}

#[derive(Deserialize)]
struct Resolve {
    #[serde(default)]
    root: String,
}

fn cargo() -> Command {
    Command::new(env::var("CARGO").as_deref().unwrap_or("cargo"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn pushd<T>(path: &Path, f: impl FnOnce() -> T) -> T {
        struct Dir(PathBuf);
        impl Drop for Dir {
            fn drop(&mut self) {
                env::set_current_dir(&self.0).unwrap();
            }
        }

        let _dir = Dir(env::current_dir().unwrap());
        env::set_current_dir(path).unwrap();
        f()
    }

    #[test]
    fn sample_modules() {
        let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../samples");
        for sample in samples.read_dir().unwrap() {
            let module = pushd(&sample.unwrap().path(), module).unwrap();

            println!("Found sample module '{}'", module.root.display());
            println!("                    '{}'", module.path.display());
        }
    }
}
