use crate as pallet_blobs;
use crate::{mock::*, *};
use codec::Encode;
use frame_support::traits::Hooks;
use frame_support::{assert_noop, assert_ok, traits::Get, BoundedVec};
use sha2::Digest;
use sp_core::{crypto::Pair, sr25519};
use sugondat_nmt::{Namespace, NmtLeaf};

fn get_blob(size: u32) -> BoundedVec<u8, <Test as pallet_blobs::Config>::MaxBlobSize> {
    vec![12u8]
        .repeat(size as usize)
        .try_into()
        .expect("provided size biggger then MaxBlobSize")
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
            namespace_id,
            blob.clone()
        ));

        assert_eq!(TotalBlobs::<Test>::get(), 1);
        assert_eq!(TotalBlobsSize::<Test>::get(), blob_len);
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
            Blobs::submit_blob(RuntimeOrigin::signed(alice()), 0, get_blob(10)),
            Error::<Test>::NoExtrinsicIndex
        );
    });
}

#[test]
fn test_max_blobs_reached() {
    let max_blobs: u32 = <Test as pallet_blobs::Config>::MaxBlobs::get();

    new_test_ext().execute_with(|| {
        let blob = get_blob(1);
        for i in 0..max_blobs {
            assert_ok!(Blobs::submit_blob(
                RuntimeOrigin::signed(alice()),
                i,
                blob.clone()
            ));
        }
    });
}

#[test]
#[should_panic = "Maximum blob limit exceeded"]
fn test_max_blobs_exceeded() {
    let max_blobs: u32 = <Test as pallet_blobs::Config>::MaxBlobs::get();

    new_test_ext().execute_with(|| {
        let blob = get_blob(1);
        for i in 0..max_blobs {
            assert_ok!(Blobs::submit_blob(
                RuntimeOrigin::signed(alice()),
                i,
                blob.clone()
            ));
        }

        let _ = Blobs::submit_blob(RuntimeOrigin::signed(alice()), 0, blob.clone());
    });
}

#[test]
fn test_max_total_blob_size_reached() {
    let max_total_blobs_size: u32 = <Test as pallet_blobs::Config>::MaxTotalBlobSize::get();
    let max_blob_size: u32 = <Test as pallet_blobs::Config>::MaxBlobSize::get();
    let blobs_needed = max_total_blobs_size / max_blob_size;

    new_test_ext().execute_with(|| {
        let blob = get_blob(max_blob_size);
        for i in 0..blobs_needed {
            assert_ok!(Blobs::submit_blob(
                RuntimeOrigin::signed(alice()),
                i,
                blob.clone()
            ));
        }
    });
}

#[test]
#[should_panic = "Maximum total blob size exceeded"]
fn test_max_total_blob_size_exceeded() {
    let max_total_blobs_size: u32 = <Test as pallet_blobs::Config>::MaxTotalBlobSize::get();
    let max_blob_size: u32 = <Test as pallet_blobs::Config>::MaxBlobSize::get();
    let blobs_needed = max_total_blobs_size / max_blob_size;

    new_test_ext().execute_with(|| {
        let blob = get_blob(max_blob_size);
        for i in 0..blobs_needed {
            assert_ok!(Blobs::submit_blob(
                RuntimeOrigin::signed(alice()),
                i,
                blob.clone()
            ));
        }
        let _ = Blobs::submit_blob(RuntimeOrigin::signed(alice()), 0, blob.clone());
    });
}

#[test]
fn test_blob_appended_to_blob_list() {
    new_test_ext().execute_with(|| {
        let blob_len = 1024;
        let blob = get_blob(blob_len);
        let blob_hash: [u8; 32] = sha2::Sha256::digest(blob.clone()).into();
        let mut blobs_metadata = vec![];

        let mut submit_blob_and_assert = |namespace_id, extrinsic_index: u32| {
            sp_io::storage::set(b":extrinsic_index", &(extrinsic_index).encode());
            assert_ok!(Blobs::submit_blob(
                RuntimeOrigin::signed(alice()),
                namespace_id,
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
                Namespace::from_u32_be(namespace_id),
                NmtLeaf {
                    extrinsic_index,
                    who: alice().into(),
                    blob_hash: blob_hash.clone(),
                },
            )
            .expect("Impossible push leaf into nmt-tree");
        };

        let mut submit_blob = |namespace_id, extrinsic_index: u32| {
            sp_io::storage::set(b":extrinsic_index", &(extrinsic_index).encode());
            assert_ok!(Blobs::submit_blob(
                RuntimeOrigin::signed(alice()),
                namespace_id,
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

        assert_eq!(TotalBlobsSize::<Test>::get(), blob_len * 4);
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
            namespace_id,
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
                Namespace::from_u32_be(extrinsic_index),
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
                    extrinsic_index,
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
