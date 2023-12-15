#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    MultiSignature,
};

pub const MAX_BLOCK_LENGTH: u32 = 5 * 1024 * 1024;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
    use super::*;
    use sp_runtime::{
        generic,
        traits::{BlakeTwo256, Hash as HashT},
    };

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
    /// Opaque block header type.
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    /// Opaque block identifier type.
    pub type BlockId = generic::BlockId<Block>;
    /// Opaque block hash type.
    pub type Hash = <BlakeTwo256 as HashT>::Output;
}

/// Invalid transaction custom errors for Sugondat Runtimes
#[repr(u8)]
pub enum InvalidTransactionCustomError {
    /// The blob exceeds the configured per-blob size limit.
    BlobExceedsSizeLimit = 100,
    /// The namespace ID is invalid.
    InvalidNamespaceId = 101,
}
