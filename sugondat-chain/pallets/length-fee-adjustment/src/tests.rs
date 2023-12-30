use crate::{
    mock::{LengthFeeAdjustment, Test},
    *,
};
use pallet_transaction_payment::Multiplier;
use sp_runtime::BuildStorage;
use sp_runtime::{traits::Get, FixedPointNumber};
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
            NextLengthMultiplierDefualt::get()
        )
    });
}

#[test]
fn test_default_target_block_size() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            TargetBlockSize::<Test>::get(),
            TargetBlockSizeDefualt::get()
        )
    });
}
