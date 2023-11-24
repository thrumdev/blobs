use base64::prelude::*;
use jsonrpsee::proc_macros::rpc;
use serde::{Deserialize, Deserializer, Serializer};

pub type JsonRPCError = jsonrpsee::types::ErrorObjectOwned;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Blob {
    #[serde(
        serialize_with = "serialize_base64",
        deserialize_with = "deserialize_base64"
    )]
    pub data: Vec<u8>,
}

fn serialize_base64<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let base64_string = BASE64_STANDARD.encode(bytes);
    serializer.serialize_str(&base64_string)
}

fn deserialize_base64<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    BASE64_STANDARD.decode(&s).map_err(serde::de::Error::custom)
}

#[rpc(server)]
pub trait RollkitRPC {
    #[method(name = "Rollkit.Retrieve")]
    async fn retrieve(&self, namespace: String, height: u64) -> Result<Vec<Blob>, JsonRPCError>;

    #[method(name = "Rollkit.Submit")]
    async fn submit(&self, namespace: String, blobs: Vec<Blob>) -> Result<u64, JsonRPCError>;
}
