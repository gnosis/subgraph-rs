//! Partial Subgraph manifest implementation.
//!
//! This module works by only partially parsing a Yaml manifest, in order to
//! read file paths to documents that need to be uploaded to IPFS in conjunction
//! with the manifest file for subgraph deployment. This partial support makes
//! it less likely to fall behind the official manifest format, for example if
//! new properties were added, they would simply be ignored here and included
//! in the final manifest.

#![allow(dead_code)]

use crate::{
    api::ipfs::CidV0,
    linker::{Linker, Resource, Source},
    mappings::Mappings,
};
use anyhow::{Context as _, Result};
use serde::{
    ser::{SerializeMap, Serializer},
    Deserialize, Serialize,
};
use serde_yaml::Value;
use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

/// A parsed Subgraph manifest.
pub struct Manifest {
    root: PathBuf,
    document: Value,
    files: Files<PathBuf>,
}

impl Manifest {
    /// Parses a subgraph manifest from bytes.
    pub fn read(path: &Path) -> Result<Self> {
        let reader = BufReader::new(File::open(path)?);
        let root = path
            .parent()
            .context("manifest file has not parent directory")?
            .canonicalize()?;
        let document = serde_yaml::from_reader::<_, Value>(reader)?;
        let files = serde_yaml::from_value(document.clone())?;

        Ok(Self {
            root,
            document,
            files,
        })
    }

    /// Links a manifest, replacing all paths with IPFS locations and returning
    /// the IPFS CID v0 hash of the uploaded manifest.
    pub fn link(self, linker: Linker, mappings: Mappings) -> Result<CidV0> {
        let Self {
            root,
            mut document,
            files,
        } = self;
        let linker = LinkAdapter { root, linker };

        // Use unchecked YAML value access here, as we already deserialized it
        // into `Files` so we know its valid.

        document["schema"]["file"] = linker.file(&files.schema.file)?;
        for (i, data_source) in files.data_sources.iter().enumerate() {
            let d_data_source = &mut document["dataSources"][i];
            d_data_source["mapping"]["file"] = linker.link(
                mappings.resolve(
                    data_source
                        .mapping
                        .file
                        .as_deref()
                        .or_else(|| mappings.default_mapping())
                        .context(
                            "More than one possible mapping Wasm modules. \
                             Try manually specifying a mapping file.",
                        )?,
                )?,
            )?;

            for (i, abi) in data_source.mapping.abis.iter().enumerate() {
                let d_abi = &mut d_data_source["mapping"]["abis"][i];
                d_abi["file"] = linker.file(&abi.file)?;
            }
        }

        linker.finish(&document)
    }
}

#[derive(Deserialize, Serialize)]
struct Files<F> {
    schema: Schema<F>,
    #[serde(rename = "dataSources")]
    data_sources: Vec<DataSource<F>>,
}

#[derive(Deserialize, Serialize)]
struct Schema<F> {
    file: F,
}

#[derive(Deserialize, Serialize)]
struct DataSource<F> {
    mapping: Mapping<F>,
}

#[derive(Deserialize, Serialize)]
struct Mapping<F> {
    abis: Vec<Abi<F>>,
    file: Option<F>,
}

#[derive(Deserialize, Serialize)]
struct Abi<F> {
    file: F,
}

struct Link(CidV0);

impl Serialize for Link {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("/", &format!("/ipfs/{}", self.0))?;
        map.end()
    }
}

struct LinkAdapter {
    root: PathBuf,
    linker: Linker,
}

impl LinkAdapter {
    fn link<S>(&self, resource: Resource<S>) -> Result<Value>
    where
        S: Source,
    {
        Ok(serde_yaml::to_value(Link(self.linker.link(resource)?))?)
    }

    fn file(&self, path: &Path) -> Result<Value> {
        self.link(Resource::file(&self.root, path))
    }

    fn finish(&self, document: &Value) -> Result<CidV0> {
        let bytes = serde_yaml::to_vec(document)?;
        self.linker.link(Resource::buffer(
            bytes.strip_prefix(b"---\n").unwrap_or(&bytes),
            Path::new("subgraph.yaml"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn deserialize_manifest() {
        let manifest =
            Manifest::read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("test/subgraph.yaml"))
                .unwrap();
        assert_eq!(manifest.files.schema.file, Path::new("schema.graphql"));
        assert_eq!(manifest.files.data_sources.len(), 1);
        assert_eq!(manifest.files.data_sources[0].mapping.abis.len(), 1);
        assert_eq!(
            manifest.files.data_sources[0].mapping.abis[0].file,
            Path::new("MyContract.abi"),
        );
        assert_eq!(
            manifest.files.data_sources[0].mapping.file.as_deref(),
            Some(Path::new("mapping.wasm")),
        );
    }

    #[test]
    #[ignore]
    fn link_manifest() {
        let manifest =
            Manifest::read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("test/subgraph.yaml"))
                .unwrap();

        let (outdir, linker) = Linker::test();
        let mappings = Mappings::from_artifacts(vec![
            Path::new(env!("CARGO_MANIFEST_DIR")).join("test/mapping.wasm")
        ])
        .unwrap();

        manifest.link(linker, mappings).unwrap();
        let linked = fs::read_to_string(outdir.path().join("subgraph.yaml")).unwrap();

        println!("Linked subgraph:\n{}", linked);
        assert_eq!(
            linked,
            fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("test/subgraph.linked.yaml")
            )
            .unwrap(),
        );
    }
}
