//! Definition of the header.

use super::Hash;
use serde::{Deserialize, Serialize};
use sugondat_nmt::TreeRoot;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Header {
    pub hash: Hash,
    pub prev_hash: Hash,
    pub nmt_root: TreeRoot,
}

impl Header {
    pub fn new(hash: Hash, prev_hash: Hash, nmt_root: TreeRoot) -> Self {
        Self {
            hash,
            prev_hash,
            nmt_root,
        }
    }
}

impl sov_rollup_interface::traits::BlockHeaderTrait for Header {
    type Hash = Hash;
    fn prev_hash(&self) -> Hash {
        self.prev_hash.clone()
    }
}

impl sov_rollup_interface::traits::CanonicalHash for Header {
    type Output = Hash;

    fn hash(&self) -> Hash {
        self.hash.clone()
    }
}
