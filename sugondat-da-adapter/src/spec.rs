use crate::types::{BlobTransaction, Hash, Header};

pub struct DaLayerSpec;

pub struct ChainParams {
    pub namespace_id: [u8; 4],
}

impl sov_rollup_interface::da::DaSpec for DaLayerSpec {
    type SlotHash = Hash;
    type BlockHeader = Header;
    type BlobTransaction = BlobTransaction;
    type InclusionMultiProof = sugondat_nmt::NamespaceProof;
    type CompletenessProof = ();
    type ChainParams = ChainParams;
}
