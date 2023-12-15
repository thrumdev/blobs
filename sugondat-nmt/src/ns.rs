use crate::NS_ID_SIZE;
use core::fmt;

/// Namespace identifier type.
///
/// Blobs are submitted into a namespace. Namespace is a 16 byte vector. Namespaces define ordering
/// lexicographically.
///
/// For convenience, a namespace can be created from an unsigned 128-bit integer. Conventionally,
/// big-endian representation of the integer is used as that is more intuitive. As one may expect:
///
/// ```
/// # use sugondat_nmt::Namespace;
/// assert!(Namespace::from_u128_be(0x0100) > Namespace::from_u128_be(0x00FF));
/// ````
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Namespace([u8; NS_ID_SIZE]);

impl Namespace {
    /// Creates a namespace from the given raw bytes.
    pub fn from_raw_bytes(raw_namespace_id: [u8; NS_ID_SIZE]) -> Self {
        Self(raw_namespace_id)
    }

    /// Returns the raw bytes of the namespace ID.
    pub fn to_raw_bytes(&self) -> [u8; NS_ID_SIZE] {
        self.0
    }

    /// A convenience function to create a namespace from an unsigned 128-bit integer.
    ///
    /// This function will take the given integer (which is assumed to be in host byte order), and
    /// take its big-endian representation as the namespace ID.
    pub fn from_u128_be(namespace_id: u128) -> Self {
        Self(namespace_id.to_be_bytes())
    }

    /// Reinterpret the namespace ID as a big-endian 128-bit integer and return.
    pub fn to_u128_be(&self) -> u128 {
        u128::from_be_bytes(self.0)
    }

    pub(crate) fn with_nmt_namespace_id(nmt_namespace_id: nmt_rs::NamespaceId<NS_ID_SIZE>) -> Self {
        Self(nmt_namespace_id.0)
    }

    pub(crate) fn nmt_namespace_id(&self) -> nmt_rs::NamespaceId<NS_ID_SIZE> {
        let mut namespace_id = nmt_rs::NamespaceId::default();
        namespace_id.0.copy_from_slice(&self.to_raw_bytes());
        namespace_id
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Print the namespace as a 16-byte hex string. We don't use `hex` crate here to avoid
        // extra dependencies.
        write!(f, "0x")?;
        for byte in self.to_raw_bytes().iter() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}
