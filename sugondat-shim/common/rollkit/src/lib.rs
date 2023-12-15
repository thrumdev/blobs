use jsonrpsee::proc_macros::rpc;

pub type JsonRPCError = jsonrpsee::types::ErrorObjectOwned;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Blob {
    #[serde(with = "sugondat_serde_util::bytes_base64")]
    pub data: Vec<u8>,
}

#[rpc(server)]
pub trait RollkitRPC {
    #[method(name = "Rollkit.Retrieve")]
    async fn retrieve(&self, namespace: String, height: u64) -> Result<Vec<Blob>, JsonRPCError>;

    #[method(name = "Rollkit.Submit")]
    async fn submit(&self, namespace: String, blobs: Vec<Blob>) -> Result<u64, JsonRPCError>;
}
