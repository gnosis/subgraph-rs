//! Module containing the graph API client implementation.

use anyhow::{bail, ensure, Result};
use curl::easy::{Easy, List};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use std::{io::Read, str};

/// A Graph HTTP client.
pub struct Client {
    node: String,
    access_token: Option<String>,
}

impl Client {
    /// Create a new Graph HTTP client connected to the specified node.
    pub fn new(node: String, access_token: Option<String>) -> Self {
        Self { node, access_token }
    }

    /// Creates a subgraph with the specified name.
    pub fn subgraph_create(&self, name: impl AsRef<str>) -> Result<Subgraph> {
        self.json_rpc(
            "subgraph_create",
            json!({
                "name": name.as_ref(),
            }),
        )
    }

    /// Creates a subgraph with the specified name.
    pub fn subgraph_deploy(
        &self,
        name: impl AsRef<str>,
        hash: impl AsRef<str>,
    ) -> Result<Deployment> {
        self.json_rpc(
            "subgraph_deploy",
            json!({
                "name": name.as_ref(),
                "ipfs_hash": hash.as_ref(),
            }),
        )
    }

    /// Perform a JSON RPC request.
    fn json_rpc<T>(&self, method: &str, params: Value) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let request_body = serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": "1337",
        }))?;
        let response_body = self.http_post(&request_body)?;
        let mut response = serde_json::from_str::<Value>(&response_body)?;

        if let Some(message) = response["error"]
            .as_object()
            .and_then(|err| err["message"].as_str())
        {
            bail!("JSON RPC error: {}", message);
        }

        let result = response["result"].take();
        ensure!(!result.is_null(), "JSON RPC missing result");
        Ok(T::deserialize(result)?)
    }

    /// Perform an HTTP POST to the specified URL.
    fn http_post(&self, body: &str) -> Result<String> {
        let mut easy = Easy::new();
        easy.url(&self.node)?;
        easy.post(true)?;
        easy.post_field_size(body.len() as _)?;
        easy.http_headers({
            let mut list = List::new();
            list.append("Content-Type: application/json").unwrap();
            list
        })?;

        if let Some(access_token) = &self.access_token {
            let _ = access_token;
            bail!("access token support not implemented");
        }

        let mut body = body.as_bytes();
        let mut buffer = Vec::new();
        {
            let mut transfer = easy.transfer();
            transfer.read_function(|buf| Ok(body.read(buf).unwrap_or(0)))?;
            transfer.write_function(|buf| {
                buffer.extend_from_slice(buf);
                Ok(buf.len())
            })?;
            transfer.perform()?;
        }

        Ok(String::from_utf8(buffer)?)
    }
}

/// A subgraph descriptor.
#[derive(Debug, Deserialize)]
pub struct Subgraph {
    /// The subgraph ID.
    pub id: String,
}

/// A subgraph deployment summary.
#[derive(Debug, Deserialize)]
pub struct Deployment {
    /// The relative URL to the GraphQL playground.
    pub playground: String,
}
