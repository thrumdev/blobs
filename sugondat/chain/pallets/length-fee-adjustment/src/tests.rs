use crate::{
    mock::{self, LengthFeeAdjustment, Test},
    *,
};
use cumulus_pallet_parachain_system::OnSystemEvent;
use pallet_transaction_payment::Multiplier;
use polkadot_primitives::{v6::PersistedValidationData, HeadData};
use sp_runtime::{assert_eq_error_rate, BuildStorage};
use sp_runtime::{
    traits::{Get, One},
    FixedPointNumber,
};
use sp_weights::{Weight, WeightToFee};

fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into()
}

#[test]
fn test_weight_to_fee() {
    new_test_ext().execute_with(|| {
        // test with multipliers smaller, equal and bigger than 1
        let multipliers = vec![
            Multiplier::from_rational(12, 19),
            Multiplier::saturating_from_integer(1),
            Multiplier::from_rational(19, 12),
        ];
        let max_block_len = <Test as Config>::MaximumBlockLength::get() as u64;
        // test 1000 step between 0 and the maximum size of a block
        for len in (0..max_block_len).step_by(max_block_len as usize / 1000) {
            for multiplier in multipliers.iter() {
                NextLengthMultiplier::<Test>::put(multiplier);

                let length_fee = len * <Test as Config>::TransactionByteFee::get();
                let expected = multiplier.saturating_mul_int(length_fee);

                assert_eq!(
                    LengthFeeAdjustment::weight_to_fee(&Weight::from_parts(len, 0)),
                    expected
                );
            }
        }
    });
}

#[test]
fn test_default_next_length_multiplier() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            NextLengthMultiplier::<Test>::get(),
            NextLengthMultiplierDefault::get()
        )
    });
}

#[test]
fn test_default_target_block_size() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            TargetBlockSize::<Test>::get(),
            TargetBlockSizeDefault::get()
        )
    });
}

#[test]
fn test_no_update_when_prev_is_zero() {
    new_test_ext().execute_with(|| {
        // using Multiplier::one() only e^(-vnt) is tested
        NextLengthMultiplier::<Test>::put(Multiplier::one());
        mock::set_last_relay_block_number(0);

        let relay_data = PersistedValidationData {
            parent_head: HeadData(vec![]),
            relay_parent_number: 100_000_000,
            relay_parent_storage_root: sp_core::H256::zero(),
            max_pov_size: 0,
        };

        LengthFeeAdjustment::on_validation_data(&relay_data);

        let mul = NextLengthMultiplier::<Test>::get();
        assert_eq!(mul, Multiplier::one());
    });
}

#[test]
fn test_skipped_block_multiplier_update() {
    new_test_ext().execute_with(|| {
        let max_skipped_blocks = <Test as Config>::MaximumSkippedBlocks::get();
        for d in (0..max_skipped_blocks).step_by(max_skipped_blocks as usize / 100) {
            // using Multiplier::one() only e^(-vnt) is tested
            NextLengthMultiplier::<Test>::put(Multiplier::one());
            mock::set_last_relay_block_number(1);

            // TODO: update with `1 + d`
            // when updating to asynchronous backing
            // https://github.com/thrumdev/blobs/issues/166
            let relay_data = PersistedValidationData {
                parent_head: HeadData(vec![]),
                relay_parent_number: 1 + 2 + d * 2, // extra 1 is because last rp was 1
                relay_parent_storage_root: sp_core::H256::zero(),
                max_pov_size: 0,
            };

            LengthFeeAdjustment::on_validation_data(&relay_data);

            let mul = NextLengthMultiplier::<Test>::get();

            // calculate expected result using f64::exp and assert on the error rate
            let target = Multiplier::from(TargetBlockSize::<Test>::get()).to_float();
            let v = <Test as Config>::AdjustmentVariableBlockSize::get().to_float();
            let expected_mul = Multiplier::from_float((-1.0 * target * v * d as f64).exp());

            //Accepted error is less than 10^(-2)
            assert_eq_error_rate!(mul, expected_mul, Multiplier::from_inner(10000000000000000));
        }
    });
}

#[test]
fn test_max_skipped_block_exceeded() {
    new_test_ext().execute_with(|| {
        NextLengthMultiplier::<Test>::put(Multiplier::one());
        mock::set_last_relay_block_number(1);

        let max_skipped_blocks = <Test as Config>::MaximumSkippedBlocks::get();
        let relay_data = PersistedValidationData {
            parent_head: HeadData(vec![]),
            // The previous relay parent was 10 times greater than the expected MaximumSkippedBlocks.
            // If the multiplier is updated with that number of skipped blocks
            // there's should be a significant divergence in the final result.
            // However, we expect it to be bounded by max_skipped_blocks.
            relay_parent_number: 1 + 2 + ((max_skipped_blocks * 10) * 2),
            relay_parent_storage_root: sp_core::H256::zero(),
            max_pov_size: 0,
        };

        LengthFeeAdjustment::on_validation_data(&relay_data);

        let mul = NextLengthMultiplier::<Test>::get();

        // calculate expected result using f64::exp and assert on the error rate
        let target = Multiplier::from(TargetBlockSize::<Test>::get()).to_float();
        let v = <Test as Config>::AdjustmentVariableBlockSize::get().to_float();
        let expected_mul =
            Multiplier::from_float((-1.0 * target * v * max_skipped_blocks as f64).exp());

        //Accepted error is less than 10^(-2)
        assert_eq_error_rate!(mul, expected_mul, Multiplier::from_inner(10000000000000000));
    });
}

#[test]
fn test_skipped_block_no_prev_data() {
    new_test_ext().execute_with(|| {
        let relay_parent_number = 156;
        let prev_multiplier = Multiplier::from(2);

        NextLengthMultiplier::<Test>::put(prev_multiplier);

        let relay_data = PersistedValidationData {
            parent_head: HeadData(vec![]),
            relay_parent_number,
            relay_parent_storage_root: sp_core::H256::zero(),
            max_pov_size: 0,
        };

        LengthFeeAdjustment::on_validation_data(&relay_data);
        assert_eq!(NextLengthMultiplier::<Test>::get(), prev_multiplier);
    });
}
