//! Namespaces are encoded as 16-byte arrays, with the following schema:
//!   - The first byte is reserved for a version byte which determines the format
//!     of the following 15 bytes. At the moment, the only supported value for this byte
//!     is `0x00`, which indicates version 0.
//!   - In version 0, bytes 1 through 5 are required to be equal to `0x00` and bytes 6 through
//!     15 are allowed to hold any value.

use core::fmt;

/// An error in namespace validation.
#[derive(Debug)]
pub enum NamespaceValidationError {
    /// Unrecognized version.
    UnrecognizedVersion(u8),
    /// V0: reserved bytes are non-zero.
    V0NonZeroReserved,
}

impl fmt::Display for NamespaceValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NamespaceValidationError::UnrecognizedVersion(version) => {
                core::write!(f, "Unrecognized version: V{}", version)
            }
            NamespaceValidationError::V0NonZeroReserved => {
                core::write!(f, "V0: reserved bytes are non-zero")
            }
        }
    }
}

/// Validate a namespace against known schemas.
pub fn validate(namespace: &[u8; 16]) -> Result<(), NamespaceValidationError> {
    if namespace[0] != 0 {
        return Err(NamespaceValidationError::UnrecognizedVersion(namespace[0]));
    }
    if &namespace[1..6] != &[0, 0, 0, 0, 0] {
        return Err(NamespaceValidationError::V0NonZeroReserved);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;
    use std::matches;

    #[quickcheck]
    fn namespace_validation_not_v0_fails(version_byte: u8) -> TestResult {
        if version_byte == 0x00 {
            return TestResult::discard();
        }
        TestResult::from_bool(matches!(
            validate(&[version_byte, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            Err(NamespaceValidationError::UnrecognizedVersion(v)) if v == version_byte,
        ))
    }

    #[quickcheck]
    fn namespace_validation_v0_reserved_occupied_fails(
        reserved: (u8, u8, u8, u8, u8),
    ) -> TestResult {
        if reserved == (0, 0, 0, 0, 0) {
            return TestResult::discard();
        }
        let (a, b, c, d, e) = reserved;
        TestResult::from_bool(matches!(
            validate(&[0u8, a, b, c, d, e, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            Err(NamespaceValidationError::V0NonZeroReserved),
        ))
    }

    #[quickcheck]
    fn namespace_validation_v0_works(namespace: Vec<u8>) -> TestResult {
        if namespace.len() < 10 {
            return TestResult::discard();
        }

        let mut n = [0u8; 16];
        n[6..].copy_from_slice(&namespace[..10]);
        TestResult::from_bool(matches!(validate(&n), Ok(())))
    }
}
