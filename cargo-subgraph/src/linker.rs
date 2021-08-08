//! Copy subgraph files to an output directly and upload to IPFS so that it can
//! be used in a subgraph deployment.

use crate::api::{
    cargo,
    ipfs::{CidV0, Client},
};
use anyhow::{ensure, Context as _, Result};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
use url::Url;

/// A linker for writing and uploading resources to IPFS for subgraph use.
pub struct Linker {
    ipfs: Client,
    outdir: PathBuf,
}

impl Linker {
    /// Creates a new resource linker from the specified IPFS base URL.
    pub fn new(ipfs_url: Url) -> Result<Self> {
        let ipfs = Client::new(ipfs_url);
        let outdir = cargo::target_directory()?
            .join("subgraph")
            .join(cargo::crate_name()?);
        fs::create_dir_all(&outdir)?;

        Ok(Self { ipfs, outdir })
    }

    #[cfg(test)]
    pub fn test() -> (tempfile::TempDir, Self) {
        let outdir = tempfile::tempdir().unwrap();
        let linker = Linker {
            ipfs: Client::new(Url::parse("http://localhost:5001").unwrap()),
            outdir: outdir.path().to_owned(),
        };

        (outdir, linker)
    }

    /// Links a resource, writing it to the output directory and uploading it to
    /// IPFS.
    pub fn link<S>(&self, resource: Resource<S>) -> Result<CidV0>
    where
        S: Source,
    {
        let output = normalize_path(&self.outdir.join(resource.name));
        ensure!(
            output.starts_with(&self.outdir),
            "linking file ends up outside of output directory",
        );
        fs::create_dir_all(output.parent().context("output path has no parent")?)?;
        resource.source.write_to_output(&output)?;

        let hash = self.ipfs.add_and_pin(&output, Some(resource.name))?;

        Ok(hash)
    }
}

/// A file for linking.
pub struct Resource<'a, S> {
    /// The path to the file on disk.
    pub source: S,
    /// A descriptive name for the file that will be used to place the file in
    /// the output directory as well as the file name when uploading to IPFS.
    pub name: &'a Path,
}

pub trait Source {
    fn write_to_output(&self, output: &Path) -> Result<()>;
}

/// A resource from a file already on disk.
pub type DiskResource<'a> = Resource<'a, PathBuf>;

impl<'a> DiskResource<'a> {
    /// Returns a new file rooted in the specified directory.
    pub fn file(root: &Path, relative: &'a Path) -> Self {
        Resource {
            source: root.join(relative),
            name: relative,
        }
    }
}

impl Source for PathBuf {
    fn write_to_output(&self, output: &Path) -> Result<()> {
        fs::copy(self, output)?;
        Ok(())
    }
}

/// A resource from an in-memory buffer.
pub type BufferedResource<'a> = Resource<'a, &'a [u8]>;

impl<'a> BufferedResource<'a> {
    /// Returns a new file rooted in the specified directory.
    pub fn buffer(contents: &'a [u8], relative: &'a Path) -> Self {
        Resource {
            source: contents,
            name: relative,
        }
    }
}

impl Source for &'_ [u8] {
    fn write_to_output(&self, output: &Path) -> Result<()> {
        fs::write(output, self)?;
        Ok(())
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut ret = PathBuf::new();
    for component in path.components() {
        match component {
            c @ Component::Prefix(..) => {
                ret = PathBuf::from(c.as_os_str());
            }
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn copy_and_upload() {
        let (outdir, linker) = Linker::test();
        let name = "test/foo.txt";
        let output = outdir.path().join(name);

        assert!(!output.exists());
        let hash = linker
            .link(Resource::file(
                Path::new(env!("CARGO_MANIFEST_DIR")),
                Path::new(name),
            ))
            .unwrap();

        assert!(output.exists());
        assert_eq!(
            hash.as_base58(),
            "QmTz3oc4gdpRMKP2sdGUPZTAGRngqjsi99BPoztyP53JMM",
        );

        println!("Linked file {} to:", name);
        println!(" - {}", output.display());
        println!(" - /ipfs/{}", hash);
    }
}
