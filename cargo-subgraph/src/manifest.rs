//! Partial Subgraph manifest implementation.
//!
//! This module works by only partially parsing a Yaml manifest, in order to
//! read file paths to documents that need to be uploaded to IPFS in conjunction
//! with the manifest file for subgraph deployment. This partial support makes
//! it less likely to fall behind the official manifest format, for example if
//! new properties were added, they would simply be ignored here and included
//! in the final manifest.

#![allow(dead_code)]

use crate::api::ipfs::CidV0;
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

    /// Sets all empty mapping files to the specified WASM module path.
    ///
    /// Note that WASM modules paths for mappings where they are specified are
    /// left untouched.
    pub fn set_mapping_path(&mut self, path: PathBuf) {
        for data_source in &mut self.files.data_sources {
            data_source.mapping.file.get_or_insert(path.clone());
        }
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

#[cfg(test)]
mod tests {
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
}
