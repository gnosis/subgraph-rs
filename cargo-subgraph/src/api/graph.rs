//! The Graph JSONRPC client API.

use crate::api::{ipfs::CidV0, jsonrpc};
use anyhow::{anyhow, Context as _, Result};
use serde::{Deserialize, Serialize};
use url::Url;

pub struct Client {
    inner: jsonrpc::Client,
}

impl Client {
    /// Creates a Graph service client with the specified URL.
    pub fn new(url: &str) -> Result<Self> {
        Ok(Self {
            inner: jsonrpc::Client::new(url)?,
        })
    }

    /// Creates a new subgraph with the specified name.
    pub fn create(&self, name: &str) -> Result<Subgraph> {
        self.inner.execute("subgraph_create", Create { name })
    }

    /// Deploys a subgraph by name and IPFS CID of the subgraph descriptor.
    pub fn deploy(&self, name: &str, cid: CidV0) -> Result<Routes> {
        let routes = self.inner.execute::<_, RawRoutes>(
            "subgraph_deploy",
            Deploy {
                name,
                ipfs_hash: cid,
            },
        )?;

        Routes::from_raw(routes, self.inner.url())
    }
}

#[derive(Serialize)]
struct Create<'a> {
    name: &'a str,
}

/// Result of creating a subgraph.
#[derive(Deserialize)]
pub struct Subgraph {
    /// The ID for the newly created subgraph.
    pub id: String,
}

#[derive(Serialize)]
struct Deploy<'a> {
    name: &'a str,
    ipfs_hash: CidV0,
}

#[derive(Deserialize)]
struct RawRoutes {
    playground: String,
    queries: String,
    subscriptions: String,
}

/// Route information for interacting with a deployed subgraph.
pub struct Routes {
    /// The URL of GraphQL playground.
    pub playground: Url,
    /// The URL for the GraphQL query API.
    pub queries: Url,
    /// The URL for the GraphQL subscription API.
    pub subscriptions: Url,
}

impl Routes {
    fn from_raw(routes: RawRoutes, base_url: &Url) -> Result<Self> {
        let playground = join_with_port(base_url, &routes.playground)?;
        let queries = join_with_port(base_url, &routes.queries)?;
        let subscriptions = {
            let mut url = join_with_port(base_url, &routes.subscriptions)?;
            url.set_scheme(match base_url.scheme() {
                "https" => "wss",
                _ => "ws",
            })
            .expect("WebSocket schemes are valid");
            url
        };

        Ok(Self {
            playground,
            queries,
            subscriptions,
        })
    }
}

fn join_with_port(url: &Url, path: &str) -> Result<Url> {
    let url = if path.starts_with(':') {
        let path_start = path
            .find('/')
            .context("relative path with port missing separator")?;

        let mut url = url.join(&path[path_start..])?;
        url.set_port(Some(path[1..path_start].parse()?))
            .map_err(|_| anyhow!("URL is an invalid base"))?;
        url
    } else {
        url.join(path)?
    };
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ipfs;
    use serde_json::json;

    #[test]
    fn serialize_create() {
        assert_eq!(
            serde_json::to_value(Create {
                name: "my/subgraph",
            })
            .unwrap(),
            json!({
                "name": "my/subgraph",
            })
        );
    }

    #[test]
    fn deserialize_create() {
        assert_eq!(
            serde_json::from_value::<Subgraph>(json!({
                "id": "my/subgraph/id",
            }))
            .unwrap()
            .id,
            "my/subgraph/id",
        );
    }

    #[test]
    fn serialize_deploy() {
        assert_eq!(
            serde_json::to_value(Deploy {
                name: "my/subgraph",
                ipfs_hash: CidV0([0; 32]),
            })
            .unwrap(),
            json!({
                "name": "my/subgraph",
                "ipfs_hash": "QmNLei78zWmzUdbeRB3CiUfAizWUrbeeZh5K1rhAQKCh51",
            })
        );
    }

    #[test]
    fn deserialize_deploy() {
        let routes = Routes::from_raw(
            serde_json::from_value(json!({
                "playground": ":81/p",
                "queries": ":81/q",
                "subscriptions": ":81/s",
            }))
            .unwrap(),
            &Url::parse("https://foo.bar:80").unwrap(),
        )
        .unwrap();
        assert_eq!(routes.playground.as_str(), "https://foo.bar:81/p");
        assert_eq!(routes.queries.as_str(), "https://foo.bar:81/q");
        assert_eq!(routes.subscriptions.as_str(), "wss://foo.bar:81/s");
    }

    #[test]
    #[ignore]
    fn create_subgraph() {
        let client = Client::new("http://localhost:8020").unwrap();
        let subgraph = client.create("my/subgraph").unwrap();

        println!("Created my/subgraph at 0x{}", subgraph.id);
    }

    #[test]
    #[ignore]
    fn deploy_subgraph() {
        let client = Client::new("http://localhost:8020").unwrap();
        let ipfs = ipfs::Client::new("http://localhost:5001").unwrap();
        let schema = ipfs
            .add_and_pin(
                "schema.graphql",
                br#"
type Empty @entity {
    id: ID!
}
                "#,
            )
            .unwrap();
        let abi = ipfs.add_and_pin("MyContract.abi", b"[]").unwrap();
        let mapping = ipfs
            .add_and_pin("MyContract.wasm", b"\0asm\x01\0\0\0")
            .unwrap();
        let manifest = ipfs
            .add_and_pin(
                "subgraph.yaml",
                format!(
                    r#"
specVersion: 0.0.2
description: My Subgraph
repository: https://github.com/my/subgraph
schema:
  file:
    /: /ipfs/{schema}
dataSources:
  - kind: ethereum/contract
    network: mainnet
    name: MyContract
    source:
      address: "0x0000000000000000000000000000000000000000"
      abi: MyContract
    mapping:
      kind: ethereum/events
      apiVersion: 0.0.4
      language: wasm/assemblyscript
      entities: []
      abis:
        - name: MyContract
          file:
            /: /ipfs/{abi}
      file:
        /: /ipfs/{mapping}
                    "#,
                    schema = schema,
                    abi = abi,
                    mapping = mapping,
                )
                .as_bytes(),
            )
            .unwrap();
        let routes = client.deploy("my/subgraph", manifest).unwrap();

        println!("Deployed my/subgraph at {}", routes.playground);
    }
}
