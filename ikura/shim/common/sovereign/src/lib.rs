use jsonrpsee::proc_macros::rpc;

#[cfg(not(any(feature = "server", feature = "client")))]
compile_error!("either feature \"server\" or \"client\" must be enabled");

pub type JsonRPCError = jsonrpsee::types::ErrorObjectOwned;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    #[serde(with = "ikura_serde_util::bytes32_hex")]
    pub block_hash: [u8; 32],
    #[serde(with = "ikura_serde_util::bytes32_hex")]
    pub prev_hash: [u8; 32],
    pub timestamp: u64,
    pub nmt_root: ikura_nmt::TreeRoot,
    pub proof: ikura_nmt::NamespaceProof,
    pub blobs: Vec<Blob>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Blob {
    #[serde(with = "ikura_serde_util::bytes32_hex")]
    pub sender: [u8; 32],
    #[serde(with = "ikura_serde_util::bytes_hex")]
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
        namespace: ikura_nmt::Namespace,
    ) -> Result<Block, JsonRPCError>;

    #[method(name = "sovereign_submitBlob")]
    async fn submit_blob(
        &self,
        blob: Vec<u8>,
        namespace: ikura_nmt::Namespace,
    ) -> Result<(), JsonRPCError>;
}
