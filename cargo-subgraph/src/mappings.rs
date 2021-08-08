//! Compile and post-process Wasm mapping modules from a crate.

#![allow(dead_code)]

use crate::{api::cargo, linker::DiskResource};
use anyhow::{anyhow, Context as _, Result};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// Collection of compiled mappings.
pub struct Mappings {
    mappings: HashMap<PathBuf, Result<PathBuf, DuplicateError>>,
}

impl Mappings {
    /// Compiles mappings from the current crate and builds a registry to be
    /// used for linking.
    pub fn compile() -> Result<Self> {
        Self::from_artifacts(cargo::build_wasm()?)
    }

    /// Returns a mappings from a collection Wasm module paths.
    pub fn from_artifacts(modules: Vec<PathBuf>) -> Result<Self> {
        let mut mappings = HashMap::new();
        for module in modules {
            let filename = Path::new(
                module
                    .file_name()
                    .context("build artifact without file name")?,
            );
            mappings
                .entry(filename.to_owned())
                .and_modify(|entry| *entry = Err(DuplicateError))
                .or_insert(Ok(module));
        }

        Ok(Self { mappings })
    }

    /// Returns the default mapping name, if there is one.
    ///
    /// A project has a default mapping if compiling the subgraph's mapping
    /// crate produces exactly one Wasm module.
    pub fn default_mapping(&self) -> Option<&Path> {
        if self.mappings.len() != 1 {
            return None;
        }
        let (filename, path) = self.mappings.iter().next()?;
        path.as_ref().ok()?;
        Some(filename)
    }

    /// Resolves a mapping module by name into a linkable resource.
    pub fn resolve<'a>(&self, name: &'a Path) -> Result<DiskResource<'a>> {
        Ok(DiskResource {
            source: self
                .mappings
                .get(name)
                .context("")?
                .as_deref()
                .map_err(|_| anyhow!("duplicate module '{}'", name.display()))?
                .to_owned(),
            name,
        })
    }
}

struct DuplicateError;
