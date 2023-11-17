#[derive(Clone, Debug)]
pub struct NmtLeaf {
	pub extrinsic_index: u32,
	pub who: [u8; 32],
	pub blob_hash: [u8; 32],
}

impl NmtLeaf {
	/// Read the NMT leaf from the given raw bytes.
	pub fn from_raw_bytes(raw: &[u8; 68]) -> Self {
		let mut extrinsic_index = [0u8; 4];
		extrinsic_index.copy_from_slice(&raw[0..4]);
		let extrinsic_index = u32::from_le_bytes(extrinsic_index);

		let mut who = [0u8; 32];
		who.copy_from_slice(&raw[4..36]);

		let mut blob_hash = [0u8; 32];
		blob_hash.copy_from_slice(&raw[36..68]);

		Self { extrinsic_index, who, blob_hash }
	}

	/// Convert the NMT leaf to raw bytes.
	pub fn to_raw_bytes(&self) -> [u8; 68] {
		let mut raw = [0u8; 68];
		raw[0..4].copy_from_slice(&self.extrinsic_index.to_le_bytes());
		raw[4..36].copy_from_slice(&self.who);
		raw[36..68].copy_from_slice(&self.blob_hash);
		raw
	}
}
