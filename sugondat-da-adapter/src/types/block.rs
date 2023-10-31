use super::{BlobTransaction, Header};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sov_rollup_interface::{services::da::SlotData, traits::CanonicalHash};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Block {
    pub header: Header,
    pub transactions: Vec<BlobTransaction>,
    pub blob_proof: sugondat_nmt::NamespaceProof,
}

impl SlotData for Block {
    type BlockHeader = Header;

    fn hash(&self) -> [u8; 32] {
        self.header.hash().0
    }

    fn header(&self) -> &Self::BlockHeader {
        &self.header
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.header == other.header
    }
}
