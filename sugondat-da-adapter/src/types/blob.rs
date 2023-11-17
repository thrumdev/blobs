use super::{Address, Hash};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sov_rollup_interface::{da::CountedBufReader, Bytes};

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
		Self { sender, hash, blob: CountedBufReader::new(Bytes::from(blob)) }
	}
}

impl sov_rollup_interface::da::BlobReaderTrait for BlobTransaction {
	type Address = Address;

	fn sender(&self) -> Address {
		self.sender.clone()
	}

	fn hash(&self) -> [u8; 32] {
		self.hash.0
	}

	fn verified_data(&self) -> &[u8] {
		self.blob.accumulator()
	}

	#[cfg(feature = "native")]
	fn advance(&mut self, num_bytes: usize) -> &[u8] {
		self.blob.advance(num_bytes);
		self.verified_data()
	}

	fn total_len(&self) -> usize {
		self.blob.total_len()
	}
}
