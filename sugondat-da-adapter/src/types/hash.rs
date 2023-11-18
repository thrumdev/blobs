//! Definition of the hash.

use serde::{Deserialize, Serialize};
use sov_rollup_interface::da::BlockHashTrait;

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct Hash(pub [u8; 32]);

impl BlockHashTrait for Hash {}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<Hash> for [u8; 32] {
    fn from(hash: Hash) -> Self {
        hash.0
    }
}
