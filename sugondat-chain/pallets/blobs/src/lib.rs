#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{DispatchInfoOf, SignedExtension},
    transaction_validity::{
        InvalidTransaction, TransactionValidity, TransactionValidityError, ValidTransaction,
    },
};
use sugondat_primitives::InvalidTransactionCustomError;

use frame_support::traits::{Get, IsSubType};

#[frame_support::pallet]
pub mod pallet {
    pub use crate::weights::WeightInfo;
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::{ValueQuery, *},
    };
    use frame_system::pallet_prelude::*;
    use sp_std::prelude::*;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The maximum number of blobs per block.
        #[pallet::constant]
        type MaxBlobs: Get<u32>;

        /// The maximum number of bytes in a blob.
        #[pallet::constant]
        type MaxBlobSize: Get<u32>;

        /// The maximum number of bytes of all blobs in a block.
        #[pallet::constant]
        type MaxTotalBlobSize: Get<u32>;

        // The weight information of this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// The total number of bytes stored in all blobs.
    #[pallet::storage]
    pub type TotalBlobSize<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// The amount of submitted blobs
    #[pallet::storage]
    pub type TotalBlobs<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Clone)]
    #[cfg_attr(test, derive(Debug, PartialEq, Eq))]
    pub struct SubmittedBlobMetadata<AccountId> {
        pub who: AccountId,
        pub extrinsic_index: u32,
        pub namespace_id: u32,
        pub blob_hash: [u8; 32],
    }

    /// The list of all submitted blobs, the size of the unbounded vector
    /// is tracked by TotalBlobs and bounded by MaxBlobs
    #[pallet::storage]
    #[pallet::unbounded]
    pub type BlobList<T: Config> =
        StorageValue<_, Vec<SubmittedBlobMetadata<T::AccountId>>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A blob was stored.
        BlobStored {
            /// Who submitted the blob.
            who: T::AccountId,
            /// The extrinsic index at which the blob was submitted.
            extrinsic_index: u32,
            /// The namespace ID the blob was submitted in.
            namespace_id: u32,
            /// The length of the blob data.
            blob_len: u32,
            /// The SHA256 hash of the blob.
            blob_hash: [u8; 32],
        },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// The extrinsic index is not available.
        NoExtrinsicIndex,
    }

    impl<T: Config> Pallet<T> {
        /// Emit a digest item containing the root of the namespace merkle tree.
        fn deposit_nmt_digest(root: sugondat_nmt::TreeRoot) {
            let bytes = root.to_raw_bytes();
            let mut digest = Vec::with_capacity(4 + bytes.len());
            digest.extend_from_slice(b"snmt");
            digest.extend_from_slice(&bytes);
            <frame_system::Pallet<T>>::deposit_log(sp_runtime::generic::DigestItem::Other(digest));
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_: BlockNumberFor<T>) -> Weight {
            // BlobList: 1r + 1w
            // TotalBlobSize: 1w
            // TotalBlobs: 1w
            // deposit_log: 1r + 1w
            T::DbWeight::get().reads_writes(2, 4)
        }

        fn on_finalize(_n: BlockNumberFor<T>) {
            TotalBlobSize::<T>::kill();
            TotalBlobs::<T>::kill();
            let blobs = BlobList::<T>::take()
                .iter()
                .map(|blob| sugondat_nmt::BlobMetadata {
                    namespace: sugondat_nmt::Namespace::from_u32_be(blob.namespace_id),
                    leaf: sugondat_nmt::NmtLeaf {
                        extrinsic_index: blob.extrinsic_index,
                        who: blob.who.encode().try_into().unwrap(),
                        blob_hash: blob.blob_hash,
                    },
                })
                .collect::<Vec<_>>();

            let root = sugondat_nmt::tree_from_blobs(blobs).root();
            Self::deposit_nmt_digest(root);
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        // The main cost is the amount of elements in the BlobList,
        // to split the cost among everyone the amount of already present element
        // is set to half of the max possible elements
        //
        // To the submit_blob weight is added a flat cost relative to the on_finalize execution
        // the amount is equal to the entire weight of on_finalized divided by 1/4 of the MaxBlobs
        // this covers perfectly the on_finalize cost if on average 1/4 of the possible blobs are submitted in one block
        //
        // Note: this PANICS if the size of the blob, the total size of all blobs, or the total number of blobs submitted
        // exceed their respective configured limits. These panics are intended to be protected against by the [`crate::PrevalidateBlobs`] extension.
        #[pallet::weight(
            T::WeightInfo::submit_blob(T::MaxBlobs::get() / 2, blob.len() as u32)
            .saturating_add(T::WeightInfo::on_finalize(0) / (T::MaxBlobs::get() / 4) as u64)
        )]
        pub fn submit_blob(
            origin: OriginFor<T>,
            namespace_id: u32,
            blob: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            let blob_len = blob.len() as u32;
            if blob_len > T::MaxBlobSize::get() {
                panic!("Blob size limit exceeded");
            }

            let Some(extrinsic_index) = <frame_system::Pallet<T>>::extrinsic_index() else {
                return Err(Error::<T>::NoExtrinsicIndex.into());
            };

            let total_blobs = TotalBlobs::<T>::get();
            if total_blobs + 1 > T::MaxBlobs::get() {
                panic!("Maximum blob limit exceeded");
            }
            TotalBlobs::<T>::put(total_blobs + 1);

            let blob_len = blob.len() as u32;
            let total_blob_size = TotalBlobSize::<T>::get();
            if total_blob_size + blob_len > T::MaxTotalBlobSize::get() {
                panic!("Maximum total blob size exceeded");
            }
            TotalBlobSize::<T>::put(total_blob_size + blob_len);

            let blob_hash = sha2_hash(&blob);

            BlobList::<T>::append(SubmittedBlobMetadata {
                extrinsic_index,
                who: who.clone(),
                namespace_id,
                blob_hash,
            });

            // Emit an event.
            Self::deposit_event(Event::<T>::BlobStored {
                who,
                extrinsic_index,
                namespace_id,
                blob_len,
                blob_hash,
            });
            Ok(().into())
        }
    }

    fn sha2_hash(data: &[u8]) -> [u8; 32] {
        use sha2::Digest;
        sha2::Sha256::digest(data).into()
    }
}

/// Prevalidates blob limits
#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct PrevalidateBlobs<T>(sp_std::marker::PhantomData<T>);

impl<T> PrevalidateBlobs<T> {
    /// Create new `SignedExtension` to prevalidate blob sizes.
    pub fn new() -> Self {
        Self(sp_std::marker::PhantomData)
    }
}

impl<T> sp_std::fmt::Debug for PrevalidateBlobs<T> {
    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        write!(f, "PrevalidateBlobs")
    }

    #[cfg(not(feature = "std"))]
    fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        Ok(())
    }
}

impl<T: Config + Send + Sync> SignedExtension for PrevalidateBlobs<T>
where
    <T as frame_system::Config>::RuntimeCall: IsSubType<Call<T>>,
{
    type AccountId = T::AccountId;
    type Call = <T as frame_system::Config>::RuntimeCall;
    type AdditionalSigned = ();
    type Pre = ();
    const IDENTIFIER: &'static str = "PrevalidateBlobs";

    fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
        Ok(())
    }

    fn pre_dispatch(
        self,
        who: &Self::AccountId,
        call: &Self::Call,
        info: &DispatchInfoOf<Self::Call>,
        len: usize,
    ) -> Result<Self::Pre, TransactionValidityError> {
        // This is what's called prior to dispatching within an actual block.

        // Check individual blob size limits
        self.validate(who, call, info, len)?;

        // Here, we return `ExhaustsResources` if the total amount or size of blobs
        // within a block is disrespected.
        //
        // This will cause honest nodes authoring blocks to skip the transaction without
        // expunging it from their transaction pool.
        if let Some(local_call) = call.is_sub_type() {
            if let Call::submit_blob { blob, .. } = local_call {
                if TotalBlobs::<T>::get() + 1 > T::MaxBlobs::get() {
                    return Err(InvalidTransaction::ExhaustsResources.into());
                }

                if TotalBlobSize::<T>::get() + blob.len() as u32 > T::MaxTotalBlobSize::get() {
                    return Err(InvalidTransaction::ExhaustsResources.into());
                }
            }
        }

        Ok(())
    }

    fn validate(
        &self,
        _who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> TransactionValidity {
        // This is what's called when evaluating transactions within the pool.

        if let Some(local_call) = call.is_sub_type() {
            if let Call::submit_blob { blob, .. } = local_call {
                if blob.len() as u32 > T::MaxBlobSize::get() {
                    // This causes the transaction to be expunged from the transaction pool.
                    // It will not be valid unless the configured limit is increased by governance,
                    // which is a rare event.
                    return Err(InvalidTransaction::Custom(
                        InvalidTransactionCustomError::BlobExceedsSizeLimit as u8,
                    )
                    .into());
                }
            }
        }
        Ok(ValidTransaction::default())
    }
}
