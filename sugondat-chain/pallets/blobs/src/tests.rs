use crate as pallet_blobs;
use crate::{mock::*, *};
use codec::Encode;
use frame_support::traits::Hooks;
use frame_support::{assert_noop, assert_ok, traits::Get};
use sha2::Digest;
use sp_core::storage::well_known_keys;
use sp_core::{crypto::Pair, sr25519};
use sp_runtime::transaction_validity::{
    InvalidTransaction, TransactionValidityError, ValidTransaction,
};
use sp_state_machine::backend::Backend;
use sp_state_machine::LayoutV1;
use sp_trie::Trie;
use sugondat_nmt::{Namespace, NmtLeaf};

fn get_blob(size: u32) -> Vec<u8> {
    vec![12u8].repeat(size as usize)
}

fn alice() -> <Test as frame_system::Config>::AccountId {
    sr25519::Pair::from_string("//Alice", None)
        .expect("Impossible generate Alice AccountId")
        .public()
        .into()
}

#[test]
fn test_correct_submitted_blob() {
    new_test_ext().execute_with(|| {
        let blob_len = 1024;
        let blob = get_blob(blob_len);
        let namespace_id = 0;

        assert_ok!(Blobs::submit_blob(
            RuntimeOrigin::signed(alice()),
            namespace_id.into(),
            blob.clone()
        ));

        assert_eq!(TotalBlobs::<Test>::get(), 1);
        assert_eq!(TotalBlobSize::<Test>::get(), blob_len);
        let blob_list = SubmittedBlobMetadata {
            who: alice(),
            extrinsic_index: 0,
            namespace_id,
            blob_hash: sha2::Sha256::digest(blob).into(),
        };
        assert_eq!(BlobList::<Test>::get().to_vec(), vec![blob_list]);
    });
}

#[test]
fn test_no_extrinsic_index() {
    new_test_ext().execute_with(|| {
        sp_io::storage::clear(b":extrinsic_index");
        assert_noop!(
            Blobs::submit_blob(RuntimeOrigin::signed(alice()), 0.into(), get_blob(10)),
            Error::<Test>::NoExtrinsicIndex
        );
    });
}

macro_rules! submit_blobs {
    ([blob_size] $blob_size: expr, [blobs_number] $n_blobs: expr) => {
        let blob = get_blob($blob_size);
        for i in 0..$n_blobs {
            assert_ok!(Blobs::submit_blob(
                RuntimeOrigin::signed(alice()),
                (i as u128).into(),
                blob.clone()
            ));
        }
    };
}

#[test]
fn test_max_blobs_reached() {
    let max_blobs: u32 = <Test as pallet_blobs::Config>::MaxBlobs::get();
    new_test_ext().execute_with(|| {
        submit_blobs!([blob_size] 1, [blobs_number] max_blobs);
    });
}

#[test]
#[should_panic = "Maximum blob limit exceeded"]
fn test_max_blobs_exceeded() {
    let max_blobs: u32 = <Test as pallet_blobs::Config>::MaxBlobs::get();
    new_test_ext().execute_with(|| {
        submit_blobs!([blob_size] 1, [blobs_number] max_blobs + 1);
    });
}

#[test]
fn test_max_total_blobs_size_reached() {
    let max_total_blobs_size: u32 = <Test as pallet_blobs::Config>::MaxTotalBlobSize::get();
    let max_blob_size: u32 = <Test as pallet_blobs::Config>::MaxBlobSize::get();
    let blobs_needed = max_total_blobs_size / max_blob_size;

    new_test_ext().execute_with(|| {
        submit_blobs!([blob_size] max_blob_size, [blobs_number] blobs_needed);
    });
}

#[test]
#[should_panic = "Maximum total blob size exceeded"]
fn test_max_total_blobs_size_exceeded() {
    let max_total_blobs_size: u32 = <Test as pallet_blobs::Config>::MaxTotalBlobSize::get();
    let max_blob_size: u32 = <Test as pallet_blobs::Config>::MaxBlobSize::get();
    let blobs_needed = max_total_blobs_size / max_blob_size;

    new_test_ext().execute_with(|| {
        submit_blobs!([blob_size] max_blob_size, [blobs_number] blobs_needed + 1);
    });
}

#[test]
fn test_blob_appended_to_blob_list() {
    new_test_ext().execute_with(|| {
        let blob_len = 1024;
        let blob = get_blob(blob_len);
        let blob_hash: [u8; 32] = sha2::Sha256::digest(blob.clone()).into();
        let mut blobs_metadata = vec![];

        let mut submit_blob_and_assert = |namespace_id: u128, extrinsic_index: u32| {
            sp_io::storage::set(b":extrinsic_index", &(extrinsic_index).encode());
            assert_ok!(Blobs::submit_blob(
                RuntimeOrigin::signed(alice()),
                namespace_id.into(),
                blob.clone()
            ));

            blobs_metadata.push(SubmittedBlobMetadata {
                who: alice(),
                extrinsic_index,
                namespace_id,
                blob_hash: blob_hash.clone(),
            });

            assert_eq!(BlobList::<Test>::get().to_vec(), blobs_metadata);
        };

        submit_blob_and_assert(1, 0);
        submit_blob_and_assert(3, 1);
        submit_blob_and_assert(2, 2);
    });
}

#[test]
fn test_namespace_order() {
    new_test_ext().execute_with(|| {
        let blob_len = 1024;
        let blob = get_blob(blob_len);
        let blob_hash: [u8; 32] = sha2::Sha256::digest(blob.clone()).into();

        let mut tree = sugondat_nmt::TreeBuilder::new();
        let mut blobs_metadata = vec![];

        let mut push_leaf = |namespace_id, extrinsic_index| {
            tree.push_leaf(
                Namespace::from_u128_be(namespace_id),
                NmtLeaf {
                    extrinsic_index,
                    who: alice().into(),
                    blob_hash: blob_hash.clone(),
                },
            )
            .expect("Impossible push leaf into nmt-tree");
        };

        let mut submit_blob = |namespace_id: u128, extrinsic_index: u32| {
            sp_io::storage::set(b":extrinsic_index", &(extrinsic_index).encode());
            assert_ok!(Blobs::submit_blob(
                RuntimeOrigin::signed(alice()),
                namespace_id.into(),
                blob.clone()
            ));

            blobs_metadata.push(SubmittedBlobMetadata {
                who: alice(),
                extrinsic_index,
                namespace_id,
                blob_hash: blob_hash.clone(),
            });
        };

        push_leaf(1, 3);
        push_leaf(1 << 8, 2);
        push_leaf(1 << 16, 1);
        push_leaf(1 << 24, 0);

        submit_blob(1 << 24, 0);
        submit_blob(1 << 16, 1);
        submit_blob(1 << 8, 2);
        submit_blob(1, 3);

        assert_eq!(TotalBlobSize::<Test>::get(), blob_len * 4);
        assert_eq!(BlobList::<Test>::get().to_vec(), blobs_metadata);

        Blobs::on_finalize(System::block_number());

        let mut logs = System::digest().logs.into_iter();
        match logs.next() {
            Some(sp_runtime::DigestItem::Other(bytes)) if bytes.starts_with(b"snmt") => {
                assert_eq!(bytes[4..], tree.root().to_raw_bytes());
            }
            _ => panic!("One DigestItem::Other should be contained in the Digest"),
        }
        // No other logs are expected
        assert_eq!(None, logs.next());
    });
}

#[test]
fn test_deposited_event() {
    new_test_ext().execute_with(|| {
        let blob_len = 50;
        let blob = get_blob(blob_len);
        let blob_hash = sha2::Sha256::digest(blob.clone()).into();
        let namespace_id = 9;
        let extrinsic_index = 15;

        // First block produced will not produce events so set the block number to 1
        System::set_block_number(1);

        sp_io::storage::set(b":extrinsic_index", &(extrinsic_index).encode());

        assert_ok!(Blobs::submit_blob(
            RuntimeOrigin::signed(alice()),
            namespace_id.into(),
            blob.clone()
        ));

        let event = Event::<Test>::BlobStored {
            who: alice(),
            extrinsic_index,
            namespace_id,
            blob_len,
            blob_hash,
        };

        System::assert_last_event(event.into());
    });
}

#[test]
fn test_on_finalize() {
    use sha2::Digest;
    use sugondat_nmt::TreeBuilder;

    let max_blobs: u32 = <Test as pallet_blobs::Config>::MaxBlobs::get();
    let mut tree = TreeBuilder::new();
    let blob = get_blob(1);
    // Counter to avoid recreating the tree from scratch everytime the loop restarts
    let mut added_leaf = 0;

    // Try on finalize 10 times
    for n_blob_to_test in (0..max_blobs).step_by((max_blobs / 10) as usize) {
        for extrinsic_index in added_leaf..n_blob_to_test {
            tree.push_leaf(
                Namespace::from_u128_be(extrinsic_index as u128),
                NmtLeaf {
                    extrinsic_index,
                    who: alice().into(),
                    blob_hash: sha2::Sha256::digest(blob.clone()).into(),
                },
            )
            .expect("Impossible push leaf into nmt-tree");
        }
        added_leaf = n_blob_to_test;

        new_test_ext().execute_with(|| {
            // prepare the state
            for extrinsic_index in 0..n_blob_to_test {
                sp_io::storage::set(b":extrinsic_index", &(extrinsic_index).encode());
                assert_ok!(Blobs::submit_blob(
                    RuntimeOrigin::signed(alice()),
                    (extrinsic_index as u128).into(),
                    blob.clone()
                ));
            }

            // Call on finalize and theck the deposited nmt root in the last event is correct
            Blobs::on_finalize(System::block_number());

            let mut logs = System::digest().logs.into_iter();
            match logs.next() {
                Some(sp_runtime::DigestItem::Other(bytes)) if bytes.starts_with(b"snmt") => {
                    assert_eq!(bytes[4..], tree.root().to_raw_bytes());
                }
                _ => panic!("One DigestItem::Other should be contained in the Digest"),
            }
            // No other logs are expected
            assert_eq!(None, logs.next());
        });
    }
}

macro_rules! submit_blob_call {
    ([blob_size] $blob_size: expr) => {
        RuntimeCall::Blobs(
            Call::submit_blob {
                namespace_id: 0.into(),
                blob: get_blob($blob_size),
            }
            .into(),
        )
    };
}

#[test]
fn test_validate_ok() {
    let prevalidate_blobs = PrevalidateBlobs::<Test>::new();

    let call = submit_blob_call!([blob_size] 1);
    assert_eq!(
        Ok(ValidTransaction::default()),
        prevalidate_blobs.validate(&alice(), &call, &Default::default(), 0)
    );
}

#[test]
fn test_validate_exceeded() {
    let prevalidate_blobs = PrevalidateBlobs::<Test>::new();

    let max_blob_size: u32 = <Test as pallet_blobs::Config>::MaxBlobSize::get();
    let call = submit_blob_call!([blob_size] max_blob_size + 1);

    assert_eq!(
        Err(TransactionValidityError::Invalid(
            InvalidTransaction::Custom(InvalidTransactionCustomError::BlobExceedsSizeLimit as u8)
        )),
        prevalidate_blobs.validate(&alice(), &call, &Default::default(), 0)
    );
}

#[test]
fn test_pre_dispatch_ok() {
    let prevalidate_blobs = PrevalidateBlobs::<Test>::new();

    new_test_ext().execute_with(|| {
        let call = submit_blob_call!([blob_size] 1);
        assert_eq!(
            Ok(()),
            prevalidate_blobs.pre_dispatch(&alice(), &call, &Default::default(), 0)
        );
    });
}

#[test]
fn test_pre_dispatch_max_blobs_exceeded() {
    let prevalidate_blobs = PrevalidateBlobs::<Test>::new();
    let max_blobs: u32 = <Test as pallet_blobs::Config>::MaxBlobs::get();

    new_test_ext().execute_with(|| {
        submit_blobs!([blob_size] 1, [blobs_number] max_blobs);

        let call = submit_blob_call!([blob_size] 1);
        assert_eq!(
            Err(InvalidTransaction::ExhaustsResources.into()),
            prevalidate_blobs.pre_dispatch(&alice(), &call, &Default::default(), 0)
        );
    });
}

#[test]
fn test_pre_dispatch_max_total_blobs_size_reached() {
    let prevalidate_blobs = PrevalidateBlobs::<Test>::new();

    let max_total_blobs_size: u32 = <Test as pallet_blobs::Config>::MaxTotalBlobSize::get();
    let max_blob_size: u32 = <Test as pallet_blobs::Config>::MaxBlobSize::get();
    let blobs_needed = max_total_blobs_size / max_blob_size;

    new_test_ext().execute_with(|| {
        submit_blobs!([blob_size] max_blob_size, [blobs_number] blobs_needed);

        let call = submit_blob_call!([blob_size] max_blob_size);
        assert_eq!(
            Err(InvalidTransaction::ExhaustsResources.into()),
            prevalidate_blobs.pre_dispatch(&alice(), &call, &Default::default(), 0)
        );
    });
}

#[test]
fn test_no_commitment_to_storage_after_finalization() {
    let mut ext = new_test_ext();

    // Execute submit_blob extrinsic and commit changes to storage
    ext.execute_with(|| {
        assert_ok!(Blobs::submit_blob(
            RuntimeOrigin::signed(alice()),
            0.into(),
            get_blob(1024)
        ));
    });
    ext.commit_all()
        .expect("Impossible to have open transactions");

    // Ensure storage items are present in the backend storage
    let assert_present_key = |key: &[u8]| match ext.as_backend().storage(key) {
        Ok(Some(_)) => (),
        _ => panic!("There must be some storage changing"),
    };
    assert_present_key(&TotalBlobSize::<Test>::hashed_key());
    assert_present_key(&TotalBlobs::<Test>::hashed_key());
    assert_present_key(&BlobList::<Test>::hashed_key());

    // Execute on_finalize and commit changes
    ext.execute_with(|| {
        Blobs::on_finalize(System::block_number());
    });
    ext.commit_all()
        .expect("Impossible to have open transactions");

    // Make sure that storage items are no longer present in the backend storage
    let assert_non_present_key = |key: &[u8]| match ext.as_backend().storage(key) {
        Ok(None) => (),
        _ => panic!("There must be no storage changing"),
    };
    assert_non_present_key(&TotalBlobSize::<Test>::hashed_key());
    assert_non_present_key(&TotalBlobs::<Test>::hashed_key());
    assert_non_present_key(&BlobList::<Test>::hashed_key());
}

#[test]
fn test_storage_proof_does_not_contain_ephemeral_keys() {
    let mut ext = new_test_ext();

    // This macro will initialize and finalize two pallets present in the runtime
    // and between execute a submit_blob extrinsic
    macro_rules! execute_submit_blob {
        () => {
            System::on_initialize(System::block_number());
            Blobs::on_initialize(System::block_number());

            assert_ok!(Blobs::submit_blob(
                RuntimeOrigin::signed(alice()),
                0.into(),
                get_blob(1024)
            ));

            Blobs::on_finalize(System::block_number());
            System::on_finalize(System::block_number());
        };
    }

    ext.execute_with(|| {
        System::set_block_number(0);
        execute_submit_blob!();
        System::set_block_number(1);
    });

    // The test is to check that nothing written in the previous blocks
    // is read from the storage, and thus the PoV has no trace
    // of pallet Blob's storage items
    let (_res, proof) = ext.execute_and_prove(|| {
        execute_submit_blob!();
    });

    // Construct the in memory trie used by validators
    let memory_bd = proof.into_memory_db();
    let root = ext.backend.root();
    let trie =
        sp_trie::TrieDBBuilder::<LayoutV1<sp_core::Blake2Hasher>>::new(&memory_bd, root).build();

    // Ensure that only the expected keys are present
    assert!(trie
        .contains(well_known_keys::EXTRINSIC_INDEX)
        .expect("trie must contain key"));

    let assert_non_present_key =
        |key: &[u8]| assert!(!trie.contains(key).expect("Impossible get value from Trie"));

    assert_non_present_key(&TotalBlobSize::<Test>::hashed_key());
    assert_non_present_key(&TotalBlobs::<Test>::hashed_key());
    assert_non_present_key(&BlobList::<Test>::hashed_key());
}
