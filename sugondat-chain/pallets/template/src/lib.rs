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

#[frame_support::pallet]
pub mod pallet {
	use sp_std::prelude::*;

	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

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
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// The total number of bytes stored in all blobs.
	#[pallet::storage]
	pub type TotalBlobsSize<T: Config> =
		StorageValue<_, u32, ValueQuery>;

	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Clone)]
	pub struct SubmittedBlobMetadata<AccountId> {
		who: AccountId,
		extrinsic_index: u32,
		namespace_id: u32,
		blob_hash: [u8; 32],
	}

	/// The list of all submitted blobs.
	#[pallet::storage]
	pub type BlobList<T: Config> =
		StorageValue<_, BoundedVec<SubmittedBlobMetadata<T::AccountId>, T::MaxBlobs>, ValueQuery>;

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
			blob_hash: [u8; 32]
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Maximum number of blobs reached.
		MaxBlobsReached,
		/// Maximum total size of blobs reached.
		MaxTotalBlobsSizeReached,
		/// The blob submitted couldn't be stored because it was too large.
		BlobTooLarge,
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
			let weight = T::DbWeight::get().reads_writes(0, 2);
			TotalBlobsSize::<T>::kill();
			BlobList::<T>::kill();
			weight
		}

		fn on_finalize(_n: BlockNumberFor<T>) {
			let blobs = BlobList::<T>::get();
			let blobs = blobs.into_iter().map(|blob| sugondat_nmt::BlobMetadata {
				namespace: sugondat_nmt::Namespace::with_namespace_id(blob.namespace_id),
				leaf: sugondat_nmt::NmtLeaf {
					extrinsic_index: blob.extrinsic_index,
					who: blob.who.encode().try_into().unwrap(),
					blob_hash: blob.blob_hash,
				},
			}).collect::<Vec<_>>();
			let root = sugondat_nmt::tree_from_blobs(blobs).root();
			Self::deposit_nmt_digest(root);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn submit_blob(
			origin: OriginFor<T>,
			namespace_id: u32,
			blob: BoundedVec<u8, T::MaxBlobSize>
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let blob_len = blob.len() as u32;
			if blob_len > T::MaxBlobSize::get() {
				return Err(Error::<T>::BlobTooLarge.into());
			}

			let Some(extrinsic_index) = <frame_system::Pallet<T>>::extrinsic_index() else {
				return Err(Error::<T>::NoExtrinsicIndex.into());
			};

			let total_blobs_size = TotalBlobsSize::<T>::get();
			if total_blobs_size + blob_len > T::MaxTotalBlobSize::get() {
				return Err(Error::<T>::MaxTotalBlobsSizeReached.into());
			}
			TotalBlobsSize::<T>::put(total_blobs_size + blob_len);

			let blob_hash = sha2_hash(&blob);

			let mut blob_list = BlobList::<T>::get();
			blob_list.try_push(SubmittedBlobMetadata {
				who: who.clone(),
				extrinsic_index,
				namespace_id,
				blob_hash,
			}).map_err(|_| Error::<T>::MaxBlobsReached)?;
			BlobList::<T>::put(blob_list);

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
