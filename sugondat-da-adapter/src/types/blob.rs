use super::{Address, Hash};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sov_rollup_interface::da::CountedBufReader;
use sov_rollup_interface::Bytes;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlobTransaction {
    pub sender: Address,
    /// Sha2 hash of the blob
    pub hash: Hash,
    pub blob: CountedBufReader<Bytes>,
}

impl BlobTransaction {
    pub fn new(sender: Address, blob: Vec<u8>) -> Self {
        use sha2::Digest;
        let hash: [u8; 32] = sha2::Sha256::digest(&blob).into();
        let hash = Hash(hash);
        Self {
            sender,
            hash,
            blob: CountedBufReader::new(Bytes::from(blob)),
        }
    }
}

impl sov_rollup_interface::da::BlobTransactionTrait for BlobTransaction {
    type Data = Bytes;
    type Address = Address;

    fn sender(&self) -> Address {
        self.sender.clone()
    }

    // Creates a new BufWithCounter structure to read the data
    fn data_mut(&mut self) -> &mut CountedBufReader<Self::Data> {
        &mut self.blob
    }

    // Creates a new BufWithCounter structure to read the data
    fn data(&self) -> &CountedBufReader<Self::Data> {
        &self.blob
    }

    fn hash(&self) -> [u8; 32] {
        self.hash.0
    }
}
