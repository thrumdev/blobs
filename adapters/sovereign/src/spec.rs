use crate::{
    types::{Address, BlobTransaction, Hash, Header},
    verifier::ChainValidityCondition,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct DaLayerSpec;

pub struct ChainParams {
    pub namespace_id: [u8; sugondat_nmt::NS_ID_SIZE],
}

impl sov_rollup_interface::da::DaSpec for DaLayerSpec {
    type SlotHash = Hash;
    type BlockHeader = Header;
    type BlobTransaction = BlobTransaction;
    type Address = Address;
    type ValidityCondition = ChainValidityCondition;
    type InclusionMultiProof = sugondat_nmt::NamespaceProof;
    type CompletenessProof = ();
    type ChainParams = ChainParams;
}
