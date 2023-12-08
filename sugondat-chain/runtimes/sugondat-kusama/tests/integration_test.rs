use polkadot_core_primitives::AccountId;
use sp_runtime::{
    generic::SignedPayload,
    traits::{Checkable, Lookup, Verify},
    transaction_validity::{InvalidTransaction, TransactionSource, TransactionValidityError},
    BuildStorage, KeyTypeId, MultiAddress, MultiSignature,
};
use sp_transaction_pool::runtime_api::TaggedTransactionQueue;

use codec::Encode;
use sp_core::{
    crypto::{key_types, Pair},
    sr25519,
};
use sp_keyring::sr25519::Keyring;
use sp_transaction_pool::runtime_api::runtime_decl_for_tagged_transaction_queue::TaggedTransactionQueueV3;
use sugondat_kusama_runtime::{
    Address, Balances, Blobs, Hash, Runtime, RuntimeCall, SignedExtra, UncheckedExtrinsic,
};
use sugondat_primitives::Signature;

use sugondat_kusama_runtime::*;

pub fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap()
        .into()
}

#[test]
fn test_validate_transaction_exceeded_max_blob_size() {
    new_test_ext().execute_with(|| {
        let alice_pair: sr25519::Pair = sr25519::Pair::from_string("//Alice", None)
            .expect("Impossible generate Alice AccountId")
            .into();

        // let alice_pair: sr25519::Pair = sr25519::Pair::from_string(
        //     "celery harvest shield father arm nice target tell regular junk miss belt",
        //     None,
        // )
        // .expect("Impossible generate Alice AccountId")
        // .into();

        let alice_account_id: <Runtime as frame_system::Config>::AccountId =
            alice_pair.public().into();
        //let alice_address = Address::Id(Keyring::Alice.to_account_id());
        let alice_address = Address::Id(alice_account_id.clone());

        let source = TransactionSource::External;

        let max_blob_size = sugondat_kusama_runtime::MaxBlobSize::get() as usize;

        let runtime_call: RuntimeCall = pallet_sugondat_blobs::Call::submit_blob {
            namespace_id: 0,
            blob: vec![0; max_blob_size + 1],
        }
        .into();

        let signed_extra: SignedExtra = (
            frame_system::CheckNonZeroSender::<Runtime>::new(),
            frame_system::CheckSpecVersion::<Runtime>::new(),
            frame_system::CheckTxVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckEra::<Runtime>::from(sp_runtime::generic::Era::Immortal),
            frame_system::CheckNonce::<Runtime>::from(0),
            frame_system::CheckWeight::<Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
            pallet_sugondat_blobs::PrevalidateBlobs::<Runtime>::new(),
        );

        let raw_payload = SignedPayload::new(runtime_call.clone(), signed_extra.clone()).unwrap();
        let signature = raw_payload.using_encoded(|payload| {
            //let sig = Keyring::Alice.sign(payload);
            let sig = alice_pair.sign(payload);
            MultiSignature::Sr25519(sig)
        });

        //let signed = dbg!(lookup.lookup(alice).unwrap()); // Inside UncheckedExtrinsic there is this lookup, maybe here something is wrong
        //if !raw_payload
        //    .using_encoded(|payload| signature.verify(payload, &(alice_account_id.into())))
        //{
        //    panic!("bad proof");
        //}
        //println!("proof OK");

        let tx =
            UncheckedExtrinsic::new_signed(runtime_call, alice_address, signature, signed_extra);

        assert_eq!(
            Err(TransactionValidityError::Invalid(
                InvalidTransaction::Custom(
                    sugondat_primitives::InvalidTransactionCustomError::BlobExceedsSizeLimit as u8
                )
            )),
            Runtime::validate_transaction(source, tx, Hash::repeat_byte(8))
        );
    });
}
