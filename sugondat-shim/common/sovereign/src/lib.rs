use jsonrpsee::proc_macros::rpc;

#[cfg(not(any(feature = "server", feature = "client")))]
compile_error!("either feature \"server\" or \"client\" must be enabled");

pub type JsonRPCError = jsonrpsee::types::ErrorObjectOwned;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    // TODO: Hash should be a newtype that serializes to hex.
    pub block_hash: [u8; 32],
    pub prev_hash: [u8; 32],
    pub timestamp: u64,
    pub nmt_root: sugondat_nmt::TreeRoot,
    pub proof: sugondat_nmt::NamespaceProof,
    pub blobs: Vec<Blob>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Blob {
    // TODO: This should be a newtype that serializes to hex.
    pub sender: [u8; 32],
    // TODO: This should be a newtype that serializes to hex.
    pub data: Vec<u8>,
}

#[cfg_attr(all(feature = "client", not(feature = "server")), rpc(client))]
#[cfg_attr(all(feature = "server", not(feature = "client")), rpc(server))]
#[cfg_attr(all(feature = "client", feature = "server"), rpc(client, server))]
pub trait SovereignRPC {
    #[method(name = "sovereign_getBlock")]
    async fn get_block(
        &self,
        height: u64,
        namespace: sugondat_nmt::Namespace,
    ) -> Result<Block, JsonRPCError>;

    #[method(name = "sovereign_submitBlob")]
    async fn submit_blob(
        &self,
        blob: Vec<u8>,
        namespace: sugondat_nmt::Namespace,
    ) -> Result<(), JsonRPCError>;
}
