//! Compile and post-process Wasm mapping modules from a crate.

use crate::{
    api::{
        cargo::{self, WasmArtifact},
        wasmopt,
    },
    linker::DiskResource,
};
use anyhow::{anyhow, bail, ensure, Context as _, Result};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use walrus::{ExportItem, Module};

/// Collection of compiled mappings.
pub struct Mappings {
    mappings: HashMap<PathBuf, Result<WasmArtifact, DuplicateError>>,
    options: MappingOpions,
}

/// Options for mappings computations.
#[derive(Default)]
pub struct MappingOpions {
    /// Optimize the Wasm module with `wasm-opt`.
    pub optimize: bool,
}

impl Mappings {
    /// Compiles mappings from the current crate and builds a registry to be
    /// used for linking.
    pub fn compile(options: MappingOpions) -> Result<Self> {
        Ok(Self::from_artifacts(cargo::build_wasm()?, options))
    }

    /// Returns a mappings from a collection Wasm module paths.
    pub fn from_artifacts(modules: Vec<WasmArtifact>, options: MappingOpions) -> Self {
        let mut mappings = HashMap::new();
        for module in modules {
            mappings
                .entry(module.name.as_str().into())
                .and_modify(|entry| *entry = Err(DuplicateError))
                .or_insert(Ok(module));
        }

        Self { mappings, options }
    }

    /// Resolves a mapping module by name into a linkable resource.
    pub fn resolve<'a>(
        &'a self,
        crate_name: &'a Path,
        api_version: &str,
    ) -> Result<DiskResource<'a>> {
        let module = self.find_module(crate_name)?;
        let source = self.post_process(module, api_version)?;
        let name = module
            .path
            .file_name()
            .context("module path without a file name")?
            .as_ref();

        Ok(DiskResource { source, name })
    }

    fn find_module(&self, crate_name: &Path) -> Result<&WasmArtifact> {
        self.mappings
            .get(crate_name)
            .with_context(|| anyhow!("cannot find mapping '{}'", crate_name.display()))?
            .as_ref()
            .map_err(|_| anyhow!("duplicate mapping '{}'", crate_name.display()))
    }

    fn post_process(&self, artifact: &WasmArtifact, api_version: &str) -> Result<PathBuf> {
        let mut module = Module::from_file(&artifact.path)?;
        let module_version = subgraph_api_version(&mut module)?;
        ensure!(
            api_version == module_version,
            "Mapping API version in manifest does not match \
             version '{}' read from Wasm module.",
            module_version,
        );

        match api_version {
            "0.0.4" => {
                subgraph_start_method(&mut module)?;
            }
            _ => bail!("Unsupported API version '{}'", api_version),
        };

        let output = artifact.path.with_extension("opt.wasm");
        module.producers.clear();
        module.emit_wasm_file(&output)?;

        if self.options.optimize {
            wasmopt::optimize_in_place(&output, &artifact.opt_level)?;
        }

        Ok(output)
    }
}

fn subgraph_api_version(module: &mut Module) -> Result<String> {
    Ok(String::from_utf8(
        module
            .customs
            .remove_raw("apiVersion")
            .context("module missing custom `apiVersion` section")?
            .data,
    )?)
}

fn subgraph_start_method(module: &mut Module) -> Result<()> {
    ensure!(module.start.is_none(), "mapping already has start function");
    let (export, function) = module
        .exports
        .iter()
        .find_map(|export| match (export.name.as_str(), &export.item) {
            ("__subgraph_start", ExportItem::Function(function)) => Some((export.id(), *function)),
            _ => None,
        })
        .context("could not find subgraph start method export")?;

    module.exports.delete(export);
    module.start = Some(function);

    Ok(())
}

struct DuplicateError;
