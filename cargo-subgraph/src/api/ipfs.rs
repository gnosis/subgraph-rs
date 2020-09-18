//! Module containing IPFS API client implementation.

use anyhow::{anyhow, ensure, Result};
use curl::easy::{Easy, Form};
use serde::Deserialize;
use serde_json::Deserializer;

/// An IPFS gateway client.
pub struct Client {
    gateway: String,
}

impl Client {
    /// Create a new Graph HTTP client connected to the specified node.
    pub fn new(mut gateway: String) -> Self {
        if gateway.ends_with('/') {
            gateway.pop();
        }

        Self { gateway }
    }

    /// Uploads and adds a file to IPFS.
    pub fn add(&self, name: &str, contents: Vec<u8>) -> Result<File> {
        let mut easy = Easy::new();
        easy.url(&format!("{}/api/v0/add", self.gateway))?;
        easy.httppost({
            let mut form = Form::new();
            form.part("file").buffer(name, contents).add()?;
            form
        })?;

        let response = perform(&mut easy)?;

        let mut file = None;
        for f in Deserializer::from_str(&response).into_iter::<File>() {
            let f = f?;
            if f.name == name {
                ensure!(
                    file.is_none(),
                    "duplicate file entry when adding '{}'",
                    name,
                );
                file = Some(f)
            }
        }

        file.ok_or_else(|| anyhow!("missing file descriptor for added file"))
    }

    /// Pins an IPFS file by hash.
    pub fn pin(&self, hash: &str) -> Result<()> {
        let mut easy = Easy::new();
        easy.url(&format!("{}/api/v0/pin/add?arg={}", self.gateway, hash))?;
        easy.post(true)?;

        let response = perform(&mut easy)?;
        let result = serde_json::from_str::<PinResult>(&response)?;
        ensure!(
            result.pins.len() == 1 && result.pins[0] == hash,
            "unexpectedly added multiple IPFS pins: {}",
            result.pins.join(", "),
        );

        Ok(())
    }
}

/// Performs an HTTP request on the specified `Easy` handle, and returning the
/// result body string.
fn perform(easy: &mut Easy) -> Result<String> {
    let mut buffer = Vec::new();
    {
        let mut transfer = easy.transfer();
        transfer.write_function(|buf| {
            buffer.extend_from_slice(buf);
            Ok(buf.len())
        })?;
        transfer.perform()?;
    }

    Ok(String::from_utf8(buffer)?)
}

/// An IPFS file.
#[derive(Debug, Deserialize)]
pub struct File {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Hash")]
    pub hash: String,
}

/// Pinning result.
#[derive(Debug, Deserialize)]
struct PinResult {
    #[serde(rename = "Pins")]
    pins: Vec<String>,
}
