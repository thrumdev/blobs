use crate::spec::DaLayerSpec;
use alloc::vec::Vec;
use digest::Digest;
use serde::{Deserialize, Serialize};
use sov_rollup_interface::{
    da::{BlockHeaderTrait, DaSpec, DaVerifier},
    zk::ValidityCondition,
};
use sugondat_nmt::{Namespace, NS_ID_SIZE};

/// A validity condition expressing that a chain of DA layer blocks is contiguous and canonical
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Hash,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
)]
pub struct ChainValidityCondition {
    pub prev_hash: [u8; 32],
    pub block_hash: [u8; 32],
}

impl ValidityCondition for ChainValidityCondition {
    type Error = anyhow::Error;
    fn combine<H: Digest>(&self, rhs: Self) -> Result<Self, Self::Error> {
        if self.block_hash != rhs.prev_hash {
            anyhow::bail!("blocks are not consequitive")
        }
        Ok(rhs)
    }
}

pub struct SugondatVerifier {
    namespace: Namespace,
}

// NOTE: this method is implemented because in the guest_sugondat (prover)
// is needed a way to create the verifier without knowing the trait DaVerifier,
// so without new method
impl SugondatVerifier {
    pub fn from_raw(raw_namespace_id: [u8; NS_ID_SIZE]) -> Self {
        Self {
            namespace: Namespace::from_raw_bytes(raw_namespace_id),
        }
    }
}

impl DaVerifier for SugondatVerifier {
    type Spec = DaLayerSpec;

    type Error = anyhow::Error;

    /// Create a new da verifier with the given chain parameters
    fn new(params: <Self::Spec as DaSpec>::ChainParams) -> Self {
        Self {
            namespace: Namespace::from_raw_bytes(params.namespace_id),
        }
    }

    // Verify that the given list of blob transactions is complete and correct.

    fn verify_relevant_tx_list(
        &self,
        block_header: &<Self::Spec as DaSpec>::BlockHeader,
        txs: &[<Self::Spec as DaSpec>::BlobTransaction],
        inclusion_proof: <Self::Spec as DaSpec>::InclusionMultiProof,
        _completeness_proof: <Self::Spec as DaSpec>::CompletenessProof,
    ) -> Result<<Self::Spec as DaSpec>::ValidityCondition, Self::Error> {
        let validity_condition = ChainValidityCondition {
            prev_hash: block_header.prev_hash().0.into(),
            block_hash: block_header.hash().0.into(),
        };

        let blob_hashes: Vec<[u8; 32]> = txs.iter().map(|tx| tx.hash.0.clone()).collect();
        let ip = inclusion_proof.verify(
            blob_hashes.as_slice(),
            block_header.nmt_root.clone(),
            self.namespace,
        );
        assert!(ip.is_ok());

        Ok(validity_condition)
    }
}
