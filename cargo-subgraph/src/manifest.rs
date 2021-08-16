//! Partial Subgraph manifest implementation.
//!
//! This module works by only partially parsing a Yaml manifest, in order to
//! read file paths to documents that need to be uploaded to IPFS in conjunction
//! with the manifest file for subgraph deployment. This partial support makes
//! it less likely to fall behind the official manifest format, for example if
//! new properties were added, they would simply be ignored here and included
//! in the final manifest.

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
    data: Data<PathBuf>,
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
        let data = serde_yaml::from_value(document.clone())?;

        Ok(Self {
            root,
            document,
            data,
        })
    }

    /// Links a manifest, replacing all paths with IPFS locations and returning
    /// the IPFS CID v0 hash of the uploaded manifest.
    pub fn link(self, linker: Linker, mappings: Mappings) -> Result<CidV0> {
        let Self {
            root,
            mut document,
            data,
        } = self;
        let linker = LinkAdapter { root, linker };

        // Use unchecked YAML value access here, as we already deserialized it
        // into `Files` so we know its valid.

        document["schema"]["file"] = linker.file(&data.schema.file)?;
        for (i, data_source) in data.data_sources.iter().enumerate() {
            let d_data_source = &mut document["dataSources"][i];

            let mapping_file = &data_source.mapping.file;
            d_data_source["mapping"]["file"] = if mapping_file.extension() == Some("wasm".as_ref())
            {
                // The subgraph is asking for a vendored Wasm file. Nothing more
                // to do!
                linker.file(mapping_file)?
            } else {
                linker.link(mappings.resolve(mapping_file, &data_source.mapping.api_version)?)?
            };

            for (i, abi) in data_source.mapping.abis.iter().enumerate() {
                let d_abi = &mut d_data_source["mapping"]["abis"][i];
                d_abi["file"] = linker.file(&abi.file)?;
            }
        }

        linker.finish(&document)
    }
}

#[derive(Deserialize)]
struct Data<F> {
    schema: Schema<F>,
    #[serde(rename = "dataSources")]
    data_sources: Vec<DataSource<F>>,
}

#[derive(Deserialize)]
struct Schema<F> {
    file: F,
}

#[derive(Deserialize)]
struct DataSource<F> {
    mapping: Mapping<F>,
}

#[derive(Deserialize)]
struct Mapping<F> {
    #[serde(rename = "apiVersion")]
    api_version: String,
    abis: Vec<Abi<F>>,
    file: F,
}

#[derive(Deserialize)]
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
    use super::*;
    use crate::{api::cargo::WasmArtifact, mappings::MappingOpions};
    use std::fs;

    #[test]
    fn deserialize_manifest() {
        let manifest =
            Manifest::read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("test/subgraph.yaml"))
                .unwrap();
        assert_eq!(manifest.data.schema.file, Path::new("schema.graphql"));
        assert_eq!(manifest.data.data_sources.len(), 2);
        assert_eq!(manifest.data.data_sources[0].mapping.api_version, "0.0.4");
        assert_eq!(manifest.data.data_sources[0].mapping.abis.len(), 1);
        assert_eq!(
            manifest.data.data_sources[0].mapping.abis[0].file,
            Path::new("MyContract.abi"),
        );
        assert_eq!(
            manifest.data.data_sources[0].mapping.file,
            Path::new("my-subgraph"),
        );
        assert_eq!(manifest.data.data_sources[1].mapping.abis.len(), 1);
        assert_eq!(manifest.data.data_sources[1].mapping.api_version, "0.0.4");
        assert_eq!(
            manifest.data.data_sources[1].mapping.abis[0].file,
            Path::new("MyContract.abi"),
        );
        assert_eq!(
            manifest.data.data_sources[1].mapping.file,
            Path::new("vendored_mapping.wasm"),
        );
    }

    #[test]
    #[ignore]
    fn link_manifest() {
        let manifest =
            Manifest::read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("test/subgraph.yaml"))
                .unwrap();

        let (outdir, linker) = Linker::test();
        let mappings = Mappings::from_artifacts(
            vec![WasmArtifact {
                name: "my-subgraph".into(),
                path: Path::new(env!("CARGO_MANIFEST_DIR")).join("test/my_subgraph.wasm"),
                opt_level: "3".to_owned(),
            }],
            MappingOpions { optimize: true },
        );

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
