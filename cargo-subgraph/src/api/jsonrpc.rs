//! Extremely simple JSONRPC-over-HTTP client implementation.

use anyhow::Result;
use curl::easy::{Easy, List};
use serde::{de::DeserializeOwned, ser::Serializer, Deserialize, Serialize};
use std::{
    error,
    fmt::{self, Display, Formatter},
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
    pub fn new(url: &str) -> Result<Self> {
        Ok(Self {
            id: AtomicU64::new(0),
            url: Url::parse(url)?,
        })
    }

    /// Returns the client's URL.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Executes the specified JSONRPC request, returning a result.
    pub fn execute<P, R>(&self, method: &str, params: P) -> Result<R>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let request = serde_json::to_string(&Request {
            jsonrpc: JsonRpcV2,
            method,
            params,
            // We don't really care about the ordering, just uniqueness.
            id: self.id.fetch_add(1, Ordering::Relaxed),
        })?;

        let response = self.execute_raw(request)?;
        let response =
            serde_json::from_str::<Response<R>>(&response).map_err(|err| -> anyhow::Error {
                match serde_json::from_str::<ErrorResponse>(&response) {
                    Ok(response) => response.error.into(),
                    Err(_) => err.into(),
                }
            })?;

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
            list.append("Content-Type: application/json; charset=utf-8")?;
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

mod id {
    use serde::ser::Serializer;

    pub fn serialize<S>(id: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&id.to_string())
    }
}

#[derive(Serialize)]
struct Request<'m, P> {
    jsonrpc: JsonRpcV2,
    method: &'m str,
    params: P,
    #[serde(with = "id")]
    id: u64,
}

#[derive(Deserialize)]
struct Response<R> {
    result: R,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: Error,
}

#[derive(Debug, Deserialize)]
struct Error {
    code: i64,
    message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::env;

    fn parse_eth(amount: &str) -> f64 {
        let amount = u128::from_str_radix(&amount[2..], 16).unwrap();
        (amount as f64) / 1e18
    }

    #[test]
    fn serialize_request() {
        assert_eq!(
            serde_json::to_value(Request {
                jsonrpc: JsonRpcV2,
                method: "subtract",
                params: [42, 23],
                id: 1,
            })
            .unwrap(),
            json!({
                "jsonrpc": "2.0",
                "method": "subtract",
                "params": [42, 23],
                "id": "1",
            }),
        );
    }

    #[test]
    fn deserialize_response() {
        assert_eq!(
            serde_json::from_value::<Response<i32>>(json!({
                "jsonrpc": "2.0",
                "result": 19,
                "id": 1,
            }))
            .unwrap()
            .result,
            19,
        );
    }

    #[test]
    fn deserialize_response_error() {
        let error = serde_json::from_value::<ErrorResponse>(json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32000,
                "message": "error",
                "data": "",
            },
            "id": "1",
        }))
        .unwrap()
        .error;

        assert_eq!(error.code, -32000);
        assert_eq!(error.message, "error");
    }

    #[test]
    #[ignore]
    fn eth_rpc() {
        let url =
            env::var("ETHEREUM_NODE_URL").expect("missing ETHEREUM_NODE_URL environment variable");
        let client = Client::new(&url).unwrap();

        let balance = client
            .execute::<_, String>(
                "eth_getBalance",
                ["0x220866B1A2219f40e72f5c628B65D54268cA3A9D", "latest"],
            )
            .unwrap();

        println!("Vitalik's Safe balance: {:.2}", parse_eth(&balance));
    }
}
