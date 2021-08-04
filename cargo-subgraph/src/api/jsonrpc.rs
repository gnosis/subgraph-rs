//! Extremely simple JSONRPC-over-HTTP client implementation.

#![allow(dead_code)]

use anyhow::{anyhow, Result};
use curl::easy::{Easy, List};
use serde::{
    de::DeserializeOwned,
    ser::{Serialize, Serializer},
};
use serde_derive::{Deserialize, Serialize};
use std::{
    io::Read,
    sync::atomic::{AtomicU64, Ordering},
};
use url::Url;

/// A JSONRPC-over-HTTP client
pub struct Client {
    id: AtomicU64,
    url: Url,
}

impl Client {
    /// Creates a new client with the specified URL.
    pub fn new(url: impl AsRef<str>) -> Result<Self> {
        Ok(Self {
            id: AtomicU64::new(0),
            url: Url::parse(url.as_ref())?,
        })
    }

    /// Executes the specified JSONRPC request, returning a result.
    pub fn execute<P, R>(&self, method: &str, params: P) -> Result<R>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let request = serde_json::to_string(&Request {
            version: JsonRpcV2,
            // We don't really care about the ordering, just uniqueness.
            id: self.id.fetch_add(1, Ordering::Relaxed),
            method,
            params,
        })?;

        let response = self.execute_raw(request)?;
        let response =
            serde_json::from_str::<Response<R>>(&response).map_err(
                |err| match serde_json::from_str::<Error>(&response) {
                    Ok(err) => anyhow!(err.error.message),
                    Err(_) => err.into(),
                },
            )?;

        Ok(response.result)
    }

    fn execute_raw(&self, request: String) -> Result<String> {
        let mut body = request.as_bytes();
        let mut buffer = Vec::new();

        let mut handle = Easy::new();
        handle.url(self.url.as_str())?;
        handle.post(true)?;
        handle.http_headers({
            let mut list = List::new();
            list.append("Content-Type: application/json")?;
            list
        })?;
        {
            let mut transfer = handle.transfer();
            transfer.read_function(|chunk| {
                Ok(body
                    .read(chunk)
                    .expect("unexpected error reading from slice"))
            })?;
            transfer.write_function(|chunk| {
                buffer.extend_from_slice(chunk);
                Ok(chunk.len())
            })?;
            transfer.perform()?;
        }

        let response = String::from_utf8(buffer)?;
        Ok(response)
    }
}

struct JsonRpcV2;

impl Serialize for JsonRpcV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("2.0")
    }
}

#[derive(Serialize)]
struct Request<'m, P> {
    version: JsonRpcV2,
    id: u64,
    method: &'m str,
    params: P,
}

#[derive(Deserialize)]
struct Response<R> {
    result: R,
}

#[derive(Deserialize)]
struct Error {
    error: ErrorData,
}

#[derive(Deserialize)]
struct ErrorData {
    code: i64,
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn parse_eth(amount: &str) -> f64 {
        let amount = u128::from_str_radix(&amount[2..], 16).unwrap();
        (amount as f64) / 1e18
    }

    #[test]
    #[ignore]
    fn eth_rpc() {
        let url =
            env::var("ETHEREUM_NODE_URL").expect("missing ETHEREUM_NODE_URL environment variable");
        let client = Client::new(url).unwrap();

        let balance = client
            .execute::<_, String>(
                "eth_getBalance",
                ["0x220866B1A2219f40e72f5c628B65D54268cA3A9D", "latest"],
            )
            .unwrap();

        println!("Vitalik's Safe balance: {:.2}", parse_eth(&balance));
    }
}
