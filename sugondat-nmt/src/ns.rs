use crate::NS_ID_SIZE;
use core::fmt;

/// The namespace. A blob is submitted into a namespace. A namespace is a 4 byte vector.
/// The convention is that the namespace id is a 4-byte little-endian integer.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Namespace(u32);

impl Namespace {
    pub fn from_raw_bytes(raw_namespace_id: [u8; 4]) -> Self {
        let namespace_id = u32::from_le_bytes(raw_namespace_id);
        Self(namespace_id)
    }

    /// Returns a namespace with the given namespace id.
    pub fn with_namespace_id(namespace_id: u32) -> Self {
        Self(namespace_id)
    }

    pub(crate) fn with_nmt_namespace_id(nmt_namespace_id: nmt_rs::NamespaceId<NS_ID_SIZE>) -> Self {
        let namespace_id = u32::from_le_bytes(nmt_namespace_id.0);
        Self(namespace_id)
    }

    pub fn namespace_id(&self) -> u32 {
        self.0
    }

    pub fn to_raw_bytes(&self) -> [u8; 4] {
        self.0.to_le_bytes()
    }

    pub(crate) fn nmt_namespace_id(&self) -> nmt_rs::NamespaceId<NS_ID_SIZE> {
        let mut namespace_id = nmt_rs::NamespaceId::default();
        namespace_id.0.copy_from_slice(&self.to_raw_bytes());
        namespace_id
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Print the namespace as a 4-byte hex string. We don't use `hex` crate here to avoid
        // extra dependencies.
        write!(f, "0x")?;
        for byte in self.to_raw_bytes().iter() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}
