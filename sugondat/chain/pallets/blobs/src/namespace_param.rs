//! Namespaces as a parameter.

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sugondat_primitives::namespace;

/// Type-safe wrapper around an unvalidated blob namespace.
#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Clone, PartialEq, RuntimeDebug)]
pub struct UnvalidatedNamespace([u8; 16]);

impl UnvalidatedNamespace {
    /// Validate the namespace, extracting the full data.
    pub fn validate(&self) -> Result<u128, namespace::NamespaceValidationError> {
        namespace::validate(&self.0).map(|()| u128::from_be_bytes(self.0))
    }
}

impl From<[u8; 16]> for UnvalidatedNamespace {
    fn from(x: [u8; 16]) -> Self {
        UnvalidatedNamespace(x)
    }
}

impl From<u128> for UnvalidatedNamespace {
    fn from(x: u128) -> Self {
        UnvalidatedNamespace(x.to_be_bytes())
    }
}
