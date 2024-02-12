use super::{BlobTransaction, Header};
use crate::verifier::ChainValidityCondition;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sov_rollup_interface::{da::BlockHeaderTrait, services::da::SlotData};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Block {
    pub header: Header,
    pub transactions: Vec<BlobTransaction>,
    pub blob_proof: ikura_nmt::NamespaceProof,
}

impl SlotData for Block {
    type BlockHeader = Header;
    type Cond = ChainValidityCondition;

    fn hash(&self) -> [u8; 32] {
        self.header.hash().0
    }

    fn header(&self) -> &Self::BlockHeader {
        &self.header
    }

    fn validity_condition(&self) -> Self::Cond {
        ChainValidityCondition {
            prev_hash: self.header.prev_hash().0.into(),
            block_hash: self.header.hash().0.into(),
        }
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.header == other.header
    }
}
