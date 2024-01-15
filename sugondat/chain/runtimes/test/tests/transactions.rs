use frame_support::{
    dispatch::GetDispatchInfo,
    traits::{fungible::Balanced, tokens::Precision},
    weights::{Weight, WeightToFee},
};
use pallet_sugondat_length_fee_adjustment::NextLengthMultiplier;
use pallet_transaction_payment::Multiplier;
use parity_scale_codec::Encode;
use sp_block_builder::runtime_decl_for_block_builder::BlockBuilderV6;
use sp_core::{crypto::Pair, sr25519};
use sp_runtime::{
    generic::SignedPayload,
    transaction_validity::{
        InvalidTransaction, TransactionSource, TransactionValidity, TransactionValidityError,
    },
    BuildStorage, FixedPointNumber, MultiSignature,
};
use sp_transaction_pool::runtime_api::runtime_decl_for_tagged_transaction_queue::TaggedTransactionQueueV3;
use sugondat_test_runtime::{
    Address, Hash, LengthFeeAdjustment, MaxBlobSize, MaxBlobs, MaxTotalBlobSize, Runtime,
    RuntimeCall, SignedExtra, UncheckedExtrinsic,
};

fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap()
        .into()
}

fn alice_pair() -> sr25519::Pair {
    sr25519::Pair::from_string("//Alice", None)
        .expect("Impossible generate Alice AccountId")
        .into()
}

// Needed to be called inside Externalities
fn prepare_environment_and_deposit_funds(account: <Runtime as frame_system::Config>::AccountId) {
    // Run a single block of the system in order to set the genesis hash.
    // The storage of `pallet_system` is initialized to hold 0x45... as the genesis
    // hash, so pushing a block with a different hash would overwrite it.
    // This ensures that the `CheckEra` and `CheckGenesis` provide the same
    // `additional_signed` payload data when constructing the transaction (here)
    // as well as validating it in `Runtime::validate_transaction`, which internally
    // calls `System::initialize` (prior to 1.5.0).
    {
        <frame_system::Pallet<Runtime>>::initialize(
            &(frame_system::Pallet::<Runtime>::block_number() + 1),
            &Hash::repeat_byte(1),
            &Default::default(),
        );
        <frame_system::Pallet<Runtime>>::finalize();
    }

    // Store some funds into the account specified as argument
    let _ = <pallet_balances::Pallet<Runtime>>::deposit(
        &account,
        100_000_000_000_000,
        Precision::BestEffort,
    )
    .expect("Impossible Store Balance");
}

// Needed to be called inside Externalities
//
// This function will only return a valid UTX if called after
// `prepare_environment_and_deposit_funds`, as certain signed extension
// operations require a storage preparation
fn create_submit_blob_utx(
    signer: sr25519::Pair,
    namespace_id: u128,
    blob: Vec<u8>,
) -> UncheckedExtrinsic {
    let signed_extra: SignedExtra = (
        frame_system::CheckNonZeroSender::<Runtime>::new(),
        frame_system::CheckSpecVersion::<Runtime>::new(),
        frame_system::CheckTxVersion::<Runtime>::new(),
        frame_system::CheckGenesis::<Runtime>::new(),
        frame_system::CheckEra::<Runtime>::from(sp_runtime::generic::Era::immortal()),
        frame_system::CheckNonce::<Runtime>::from(0),
        frame_system::CheckWeight::<Runtime>::new(),
        pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
        pallet_sugondat_blobs::PrevalidateBlobs::<Runtime>::new(),
    );

    let runtime_call: RuntimeCall = pallet_sugondat_blobs::Call::submit_blob {
        namespace_id: namespace_id.into(),
        blob,
    }
    .into();

    let raw_payload = SignedPayload::new(runtime_call.clone(), signed_extra.clone()).unwrap();
    let signature = MultiSignature::Sr25519(signer.sign(&raw_payload.encode()));

    UncheckedExtrinsic::new_signed(
        runtime_call,
        Address::Id(signer.public().into()),
        signature,
        signed_extra,
    )
}

fn test_validate_transaction(blob_size: u32, assertion: impl FnOnce(TransactionValidity)) {
    new_test_ext().execute_with(|| {
        prepare_environment_and_deposit_funds(alice_pair().public().into());

        let utx = create_submit_blob_utx(alice_pair(), 0, vec![0; blob_size as usize]);

        let res =
            Runtime::validate_transaction(TransactionSource::External, utx, Hash::repeat_byte(8));
        assertion(res);
    });
}

#[test]
fn test_validate_transaction_ok() {
    test_validate_transaction(MaxBlobSize::get(), |res| assert!(res.is_ok()))
}

#[test]
fn test_validate_transaction_max_blob_size_exceeded() {
    test_validate_transaction(MaxBlobSize::get() + 1, |res| {
        assert_eq!(
            res,
            Err(TransactionValidityError::Invalid(
                InvalidTransaction::Custom(
                    sugondat_primitives::InvalidTransactionCustomError::BlobExceedsSizeLimit as u8,
                )
            ))
        )
    });
}

fn test_pre_dispatch(modify_storage: impl FnOnce()) {
    new_test_ext().execute_with(|| {
        modify_storage();

        prepare_environment_and_deposit_funds(alice_pair().public().into());

        let utx = create_submit_blob_utx(alice_pair(), 0, vec![0; 10]);

        assert_eq!(
            Runtime::apply_extrinsic(utx),
            Err(TransactionValidityError::Invalid(
                InvalidTransaction::ExhaustsResources,
            ))
        )
    });
}

#[test]
fn test_pre_dispatch_max_blobs_exceeded() {
    test_pre_dispatch(|| pallet_sugondat_blobs::TotalBlobs::<Runtime>::put(MaxBlobs::get()));
}

#[test]
fn test_pre_dispatch_max_total_blob_size_exceeded() {
    test_pre_dispatch(|| {
        pallet_sugondat_blobs::TotalBlobSize::<Runtime>::put(MaxTotalBlobSize::get())
    });
}

#[test]
fn test_inclusion_fee_uses_length_to_fee() {
    // Test that inclusion fee is evaluated properly
    // by the transaction_payment pallet
    new_test_ext().execute_with(|| {
        let call: RuntimeCall = pallet_sugondat_blobs::Call::submit_blob {
            namespace_id: 0.into(),
            blob: vec![0; 1],
        }
        .into();

        NextLengthMultiplier::<Runtime>::put(&Multiplier::saturating_from_rational(1, 12));

        let inclusion_fee_zero_length = pallet_transaction_payment::Pallet::<Runtime>::compute_fee(
            0,
            &call.get_dispatch_info(),
            0,
        );

        let inclusion_fee = pallet_transaction_payment::Pallet::<Runtime>::compute_fee(
            call.size_hint() as u32,
            &call.get_dispatch_info(),
            0,
        );

        let length_fee = inclusion_fee - inclusion_fee_zero_length;

        let expected_length_fee =
            LengthFeeAdjustment::weight_to_fee(&Weight::from_parts(call.size_hint() as u64, 0));

        assert_eq!(length_fee, expected_length_fee);
    });
}
