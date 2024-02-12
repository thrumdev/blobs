pub use crate::ns::Namespace;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TreeRoot {
    #[cfg_attr(feature = "serde", serde(with = "ikura_serde_util::bytes32_hex"))]
    pub root: [u8; 32],
    pub min_ns: Namespace,
    pub max_ns: Namespace,
}

impl TreeRoot {
    pub fn from_raw_bytes(raw: &[u8; 68]) -> Self {
        let mut root = [0u8; 32];
        root.copy_from_slice(&raw[0..32]);

        let min_ns = Namespace::from_raw_bytes(raw[32..48].try_into().unwrap());
        let max_ns = Namespace::from_raw_bytes(raw[48..64].try_into().unwrap());

        Self {
            root,
            min_ns,
            max_ns,
        }
    }

    pub fn to_raw_bytes(&self) -> [u8; 68] {
        let mut raw = [0u8; 68];
        raw[0..32].copy_from_slice(&self.root);
        raw[32..48].copy_from_slice(&self.min_ns.to_raw_bytes());
        raw[48..64].copy_from_slice(&self.max_ns.to_raw_bytes());
        raw
    }
}
