//! Benchmarking setup for pallet-ikura-blobs
use super::*;

#[allow(unused)]
use crate::Pallet as Blobs;
use frame_benchmarking::__private::traits::Hooks;
#[allow(unused)]
use frame_benchmarking::v2::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::traits::Get;
use frame_system::RawOrigin;
use parity_scale_codec::Encode;
use sp_std::vec;

// Command to run the benchmarks:
// ./target/release/ikura-node benchmark pallet \
// --dev \
// --pallet pallet_ikura_blobs \
// --extrinsic '*' \
// --steps 20 \
// --repeat 20 \
// --template <path_to_weight_template_file>.hbs \
// --output ikura-chain/pallets/blobs/src/weights.rs

#[benchmarks]
mod benchmarks {
    use super::*;

    fn init_state<T: Config>(caller: T::AccountId, x: u32) {
        for ext_index in 0..x {
            sp_io::storage::set(b":extrinsic_index", &(ext_index).encode());
            Blobs::<T>::submit_blob(
                RawOrigin::Signed(caller.clone()).into(),
                (ext_index as u128).into(),
                ext_index.to_le_bytes().to_vec(),
            )
            .expect("Preparation Extrinsic failed");
        }
    }

    #[benchmark]
    // x represent the amount of SubmittedBlobMetadata already stored in BlobList
    // while y is the size of the blob in bytes
    fn submit_blob(
        x: Linear<0, { T::MaxBlobs::get() - 1 }>,
        y: Linear<1, { T::MaxBlobSize::get() }>,
    ) {
        let caller: T::AccountId = whitelisted_caller();

        // the values in the submitted data are not important so
        // the ext_index will be used as namespace_id and as blob
        init_state::<T>(caller.clone(), x);
        sp_io::storage::set(b":extrinsic_index", &(x).encode());

        // Create a random blob that needs to be hashed on chain
        let blob = vec![23u8; y as usize];
        #[extrinsic_call]
        _(RawOrigin::Signed(caller), 0.into(), blob);

        // Check that an item is inserted in the BlobList and
        // the new value stored in TotalBlobs are correct
        assert_eq!(BlobList::<T>::get().len(), x as usize + 1);
        assert_eq!(TotalBlobs::<T>::get(), x as u32 + 1);
        // the blob size of all the previus sumbitted blob si 4 bytes (u32.to_le_bytes())
        assert_eq!(TotalBlobSize::<T>::get(), (x * 4) + y);
    }

    #[benchmark]
    // x represent the amount of SubmittedBlobMetadata already stored in BlobList
    fn on_finalize(x: Linear<0, { T::MaxBlobs::get() }>) {
        let caller: T::AccountId = whitelisted_caller();

        init_state::<T>(caller.clone(), x);

        #[block]
        {
            Blobs::<T>::on_finalize(15u32.into());
        }

        // Every storage Item should be killed
        assert_eq!(BlobList::<T>::get().len(), 0);
        assert_eq!(TotalBlobs::<T>::get(), 0);
        assert_eq!(TotalBlobSize::<T>::get(), 0);
    }

    impl_benchmark_test_suite!(Blobs, crate::mock::new_test_ext(), crate::mock::Test);
}
