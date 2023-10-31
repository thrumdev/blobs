use crate::spec::DaLayerSpec;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sov_rollup_interface::crypto::SimpleHasher;
use sov_rollup_interface::da::{DaSpec, DaVerifier};
use sov_rollup_interface::traits::{BlockHeaderTrait, CanonicalHash};
use sov_rollup_interface::zk::traits::ValidityCondition;
use sugondat_nmt::Namespace;

/// A validity condition expressing that a chain of DA layer blocks is contiguous and canonical
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct ChainValidityCondition {
    pub prev_hash: [u8; 32],
    pub block_hash: [u8; 32],
}

impl ValidityCondition for ChainValidityCondition {
    type Error = anyhow::Error;
    fn combine<H: SimpleHasher>(&self, rhs: Self) -> Result<Self, Self::Error> {
        if self.block_hash != rhs.prev_hash {
            anyhow::bail!("blocks are not consequitive")
        }
        Ok(rhs)
    }
}

pub struct SugondatVerifier {
    namespace: Namespace,
}

impl DaVerifier for SugondatVerifier {
    type Spec = DaLayerSpec;

    type Error = anyhow::Error;

    /// Any conditions imposed by the DA layer which need to be checked outside of the SNARK
    type ValidityCondition = ChainValidityCondition;

    /// Create a new da verifier with the given chain parameters
    fn new(params: <Self::Spec as DaSpec>::ChainParams) -> Self {
        Self {
            namespace: Namespace::from_raw_bytes(params.namespace_id),
        }
    }

    // Verify that the given list of blob transactions is complete and correct.
    fn verify_relevant_tx_list<H: SimpleHasher>(
        &self,
        block_header: &<Self::Spec as sov_rollup_interface::da::DaSpec>::BlockHeader,
        txs: &[<Self::Spec as sov_rollup_interface::da::DaSpec>::BlobTransaction],
        inclusion_proof: <Self::Spec as sov_rollup_interface::da::DaSpec>::InclusionMultiProof,
        _completeness_proof: <Self::Spec as sov_rollup_interface::da::DaSpec>::CompletenessProof,
    ) -> Result<ChainValidityCondition, Self::Error> {
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
