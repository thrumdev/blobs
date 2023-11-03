//! Definition of the header.

use super::Hash;
use serde::{Deserialize, Serialize};
use sov_rollup_interface::da::{BlockHeaderTrait, Time};
use sugondat_nmt::TreeRoot;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Header {
    pub hash: Hash,
    pub prev_hash: Hash,
    pub nmt_root: TreeRoot,
    pub height: u64,
    pub time: Time,
}

impl Header {
    pub fn new(
        hash: Hash,
        prev_hash: Hash,
        nmt_root: TreeRoot,
        height: u64,
        timestamp: u64,
    ) -> Self {
        Self {
            hash,
            prev_hash,
            nmt_root,
            height,
            // timestamp is in milliseconds so / 1000
            time: Time::from_secs((timestamp / 1000) as i64),
        }
    }
}

impl BlockHeaderTrait for Header {
    type Hash = Hash;

    fn prev_hash(&self) -> Self::Hash {
        self.prev_hash.clone()
    }

    fn hash(&self) -> Self::Hash {
        self.hash.clone()
    }

    fn height(&self) -> u64 {
        self.height
    }

    fn time(&self) -> Time {
        self.time.clone()
    }
}
