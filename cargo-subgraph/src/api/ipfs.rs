//! Simple bare-bones IPFS client implementation.

use anyhow::{ensure, Context as _, Result};
use curl::easy::{Easy, Form};
use serde::{
    de::{self, Deserializer, Visitor},
    ser::Serializer,
    Deserialize, Serialize,
};
use std::{
    fmt::{self, Debug, Display, Formatter},
    path::{Path, PathBuf},
    str,
};
use url::Url;

/// CID v0.
///
/// The bytes are the Sha-256 hash of the data being identified.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct CidV0(pub [u8; 32]);

impl CidV0 {
    /// Parses the base58 string and returns a CID.
    pub fn from_base58(s: &str) -> Result<Self> {
        ensure!(&s[..2] == "Qm", "missing CID v0 0x1220 prefix");
        let mut buf = [0u8; 34];
        bs58::decode(s).into(&mut buf)?;
        let mut digest = [0u8; 32];
        digest.copy_from_slice(&buf[2..]);
        Ok(Self(digest))
    }

    /// Returns the base58 representation of the CID.
    pub fn as_base58(&self) -> String {
        let mut buf = [0u8; 34];
        buf[..2].copy_from_slice(b"\x12\x20");
        buf[2..].copy_from_slice(&self.0);
        bs58::encode(buf).into_string()
    }
}

impl Debug for CidV0 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&self.as_base58())
    }
}

impl Display for CidV0 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&self.as_base58())
    }
}

impl<'de> Deserialize<'de> for CidV0 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CidV0Visitor;
        impl Visitor<'_> for CidV0Visitor {
            type Value = CidV0;

            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                f.write_str("base58 encoded CID v0 ('Qm...')")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                CidV0::from_base58(v).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(CidV0Visitor)
    }
}

impl Serialize for CidV0 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.as_base58())
    }
}

/// Simple IPFS client that can add and pin file blobs.
pub struct Client {
    base_url: Url,
}

impl Client {
    /// Creates a new client with the specified base URL.
    pub fn new(base_url: &str) -> Result<Self> {
        Ok(Self {
            base_url: Url::parse(base_url)?,
        })
    }

    fn url(&self, path: &str) -> Result<Url> {
        Ok(self.base_url.join(path)?)
    }

    /// Adds and pins a file to IPFS returning its CID.
    pub fn add_and_pin(&self, file: &Path, filename: &Path) -> Result<CidV0> {
        let added = self.add(file, filename)?;
        let cid = added
            .into_iter()
            .find(|file| file.name == filename)
            .context("file missing from added list")?
            .hash;
        let pinned = self.pin(cid)?;
        pinned
            .into_iter()
            .find(|pin| *pin == cid)
            .context("file missing from pinned list")?;

        Ok(cid)
    }

    /// Adds a new file to IPFS.
    pub fn add(&self, file: &Path, filename: &Path) -> Result<Vec<Add>> {
        let mut buffer = Vec::new();

        let mut handle = Easy::new();
        handle.url(self.url("api/v0/add")?.as_str())?;
        handle.httppost({
            let mut form = Form::new();
            form.part("file")
                .file(file)
                .filename(filename)
                .content_type("application/octet-stream")
                .add()?;
            form
        })?;
        {
            let mut transfer = handle.transfer();
            transfer.write_function(|chunk| {
                buffer.extend_from_slice(chunk);
                Ok(chunk.len())
            })?;
            transfer.perform()?;
        }

        Ok(serde_json::Deserializer::from_slice(&buffer)
            .into_iter::<Add>()
            .collect::<Result<_, _>>()?)
    }

    /// Pins a file to IPFS
    pub fn pin(&self, cid: CidV0) -> Result<Vec<CidV0>> {
        let mut buffer = Vec::new();

        let mut handle = Easy::new();
        handle.url(
            {
                let mut url = self.url("api/v0/pin/add")?;
                url.query_pairs_mut().append_pair("arg", &cid.as_base58());
                url
            }
            .as_str(),
        )?;
        handle.post(true)?;
        {
            let mut transfer = handle.transfer();
            transfer.write_function(|chunk| {
                buffer.extend_from_slice(chunk);
                Ok(chunk.len())
            })?;
            transfer.perform()?;
        }

        Ok(serde_json::from_slice::<Pins>(&buffer)?.pins)
    }
}

#[derive(Deserialize)]
pub struct Add {
    #[serde(rename = "Name")]
    pub name: PathBuf,
    #[serde(rename = "Hash")]
    pub hash: CidV0,
}

#[derive(Deserialize)]
struct Pins {
    #[serde(rename = "Pins")]
    pins: Vec<CidV0>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cid_from_base58() {
        assert_eq!(
            &CidV0::from_base58("QmY7Yh4UquoXHLPFo2XbhXkhBvFoPwmQUSa92pxnxjQuPU")
                .unwrap()
                .0,
            b"\x91\x39\x83\x9e\x65\xfa\xbe\xa9\xef\xd2\x30\x89\x8a\xd8\xb5\x74\
              \x50\x91\x47\xe4\x8d\x7c\x1e\x87\xa3\x3d\x6d\xa7\x0f\xd2\xef\xbf",
        );
    }

    #[test]
    fn cid_as_base58() {
        assert_eq!(
            CidV0(
                *b"\x91\x39\x83\x9e\x65\xfa\xbe\xa9\xef\xd2\x30\x89\x8a\xd8\xb5\x74\
                   \x50\x91\x47\xe4\x8d\x7c\x1e\x87\xa3\x3d\x6d\xa7\x0f\xd2\xef\xbf"
            )
            .as_base58(),
            "QmY7Yh4UquoXHLPFo2XbhXkhBvFoPwmQUSa92pxnxjQuPU",
        );
    }

    #[test]
    fn cid_serialization() {
        let base58 = json!("QmNLei78zWmzUdbeRB3CiUfAizWUrbeeZh5K1rhAQKCh51");
        assert_eq!(serde_json::to_value(CidV0([0; 32])).unwrap(), base58);
        assert_eq!(serde_json::from_value::<CidV0>(base58).unwrap().0, [0; 32]);
    }

    #[test]
    #[ignore]
    fn add_and_pin() {
        let client = Client::new("http://localhost:5001").unwrap();
        let cid = client
            .add_and_pin(
                &Path::new(env!("CARGO_MANIFEST_DIR")).join("test/foo.txt"),
                Path::new("foo.txt"),
            )
            .unwrap();

        println!("Added and pinned {}", cid);
    }
}
