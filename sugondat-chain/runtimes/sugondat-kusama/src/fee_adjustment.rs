use crate::constants::kusama::currency::MILLICENTS;
use frame_support::parameter_types;
use pallet_transaction_payment::{Multiplier, MultiplierUpdate, TargetedFeeAdjustment};
use sp_runtime::{
    traits::{Bounded, Convert},
    FixedPointNumber, Perquintill, SaturatedConversion, Saturating,
};
use sp_weights::Weight;
use sugondat_primitives::{Balance, MAXIMUM_BLOCK_LENGTH};

parameter_types! {
    /// Relay Chain `TransactionByteFee` / 10
    pub const TransactionByteFee: Balance = MILLICENTS;

    // parameters used by BlobsFeeAdjustment
    // to update NextFeeMultiplier and NextLengthMultiplier
    //
    // Common constants used in all runtimes for SlowAdjustingFeeUpdate
    /// The portion of the `NORMAL_DISPATCH_RATIO` that we adjust the fees with. Blocks filled less
    /// than this will decrease the weight and more will increase.
    pub storage TargetBlockFullness: Perquintill = Perquintill::from_percent(25);

    /// The adjustment variable of the runtime. Higher values will cause `TargetBlockFullness` to
    /// change the fees more rapidly.
    pub AdjustmentVariableBlockFullness: Multiplier = Multiplier::saturating_from_rational(75, 1_000_000);
    /// that combined with `AdjustmentVariable`, we can recover from the minimum.
    /// See `multiplier_can_grow_from_zero`.
    pub MinimumMultiplierBlockFullness: Multiplier = Multiplier::saturating_from_rational(1, 10u128);
    /// The maximum amount of the multiplier.
    pub MaximumMultiplierBlockFullness: Multiplier = Bounded::max_value();


    pub storage NextLengthMultiplier: Multiplier = Multiplier::saturating_from_integer(1);
    pub storage TargetBlockSize: Perquintill = Perquintill::from_percent(16); // 0.8MiB
    //  v = p / k * (1 - s*) = 0.3 / (300 * (1 - 0.16))
    //  at most 30% (=p) fees variation in one hour, 300 blocks (=k)
    pub AdjustmentVariableBlockSize: Multiplier = Multiplier::saturating_from_rational(1, 840);
    // TODO: decide the value of MinimumMultiplierBlockSize, https://github.com/thrumdev/blobs/issues/154
    pub MinimumMultiplierBlockSize: Multiplier = Multiplier::saturating_from_rational(1, 10u128);
    pub MaximumMultiplierBlockSize: Multiplier = Bounded::max_value();
}

/// Currently pallet_transaction_payment use the following formula:
///
/// ```ignore
/// inclusion_fee = base_fee + length_fee + [targeted_fee_adjustment * weight_fee];
/// ```
///
/// Letting us able to update `targeted_fee_adjustment` at the end of each block
/// thanks to `FeeMultiplierUpdate`, this associated type is called inside the `on_finalize`
/// of the transaction_payment pallet with the aim of converting the before `targeted_fee_adjustment`
/// to a new one based on the congestion of the network
///
/// What this struct does is this PLUS a side effect, the goal is to reach a different formula to
/// calculate fees:
///
/// ```ignore
/// inclusion_fee = base_fee + [targeted_length_fee_adjustment * length_fee] + [targeted_weight_fee_adjustment * weight_fee];
/// ```
///
/// As you can see `targeted_fee_adjustment` becomes `targeted_weight_fee_adjustment` but the behavior
/// remains the same, the side effect is the changing to the value `targeted_length_fee_adjustment`,
/// this formula is achievable because inside pallet_transaction_payment the function `compute_fee_raw`
/// that just computes the final fee associated with an extrinsic uses the associated type `LengthToFee`
/// that converts the length of an extrinsic to a fee.
///
/// By default the implementation is a constant multiplication but we want to achieve a dynamic formula
/// that can adapt based on the usage of the network, this can't solely be done by this struct but needs
/// to be bundled with a custom implementation of `LengthToFee`.
///
/// This struct ONLY provide a dynamic update of `targeted_length_fee_adjustment` and `targeted_weight_fee_adjustment`
/// based on the congestion and usage of the blocks, while the formula si effectively implemented like
/// explained above only thanks to `LengthToFee`
pub struct BlobsFeeAdjustment<T: frame_system::Config>(core::marker::PhantomData<T>);

impl<T: frame_system::Config> Convert<Multiplier, Multiplier> for BlobsFeeAdjustment<T>
where
    T: frame_system::Config,
{
    /// This function should be a pure function used to update NextFeeMultiplier
    /// but will also has the side effect of update NextLengthMultiplier
    fn convert(previous_fee_multiplier: Multiplier) -> Multiplier {
        // Update NextLengthMultiplier

        // To update the value will be used the same formula as TargetedFeeAdjustment,
        // described here: https://research.web3.foundation/Polkadot/overview/token-economics#2-slow-adjusting-mechanism
        //
        // so this is mainly a copy paste of that function because it works on normalized mesurments,
        // so if it is ref_time, proof_size or length of the extrinsic the mutliplier will be evaluated properly.
        // The main problem is that TargetedFeeAdjustment::convert uses directly a call to the storage to extract
        // the weight of the current block so there is no way to pass the length as input argument,
        // here I will copy paste all the needed part to update properly NextLengthMultiplier

        // Defensive only. The multiplier in storage should always be at most positive. Nonetheless
        // we recover here in case of errors, because any value below this would be stale and can
        // never change.

        let previous_len_multiplier = NextLengthMultiplier::get();
        let min_multiplier = MinimumMultiplierBlockSize::get();
        let max_multiplier = MaximumMultiplierBlockSize::get();
        let previous_len_multiplier = previous_len_multiplier.max(min_multiplier);

        // Pick the limiting dimension. (from TargetedFeeAdjustment::convert)
        //
        // In this case it is the length of all extrinsic, always
        let (normal_limiting_dimension, max_limiting_dimension) = (
            <frame_system::Pallet<T>>::all_extrinsics_len(),
            MAXIMUM_BLOCK_LENGTH as u64,
        );

        let target_block_size = TargetBlockSize::get();
        let adjustment_variable = AdjustmentVariableBlockSize::get();

        let target_size = (target_block_size * max_limiting_dimension) as u128;
        let block_size = normal_limiting_dimension as u128;

        // determines if the first_term is positive
        let positive = block_size >= target_size;
        let diff_abs = block_size.max(target_size) - block_size.min(target_size);

        // defensive only, a test case assures that the maximum weight diff can fit in Multiplier
        // without any saturation.
        let diff = Multiplier::saturating_from_rational(diff_abs, max_limiting_dimension.max(1));
        let diff_squared = diff.saturating_mul(diff);

        let v_squared_2 = adjustment_variable.saturating_mul(adjustment_variable)
            / Multiplier::saturating_from_integer(2);

        let first_term = adjustment_variable.saturating_mul(diff);
        let second_term = v_squared_2.saturating_mul(diff_squared);

        let new_len_multiplier = if positive {
            let excess = first_term
                .saturating_add(second_term)
                .saturating_mul(previous_len_multiplier);
            previous_len_multiplier
                .saturating_add(excess)
                .clamp(min_multiplier, max_multiplier)
        } else {
            // Defensive-only: first_term > second_term. Safe subtraction.
            let negative = first_term
                .saturating_sub(second_term)
                .saturating_mul(previous_len_multiplier);
            previous_len_multiplier
                .saturating_sub(negative)
                .clamp(min_multiplier, max_multiplier)
        };

        NextLengthMultiplier::set(&new_len_multiplier);

        // Update NextFeeMultiplier
        //
        // Here is the tricky part, this method return the new value associated with
        // NextFeeMultiplier (in the old fashion) because weight dynamic adjustment is battle tested
        // while previously have updated the `NextLengthMultiplier` used in `LengthToWeight`
        TargetedFeeAdjustment::<
            T,
            TargetBlockFullness,
            AdjustmentVariableBlockFullness,
            MinimumMultiplierBlockFullness,
            MaximumMultiplierBlockFullness,
        >::convert(previous_fee_multiplier)
    }
}

impl<T: frame_system::Config> MultiplierUpdate for BlobsFeeAdjustment<T> {
    fn min() -> Multiplier {
        MinimumMultiplierBlockFullness::get()
    }
    fn max() -> Multiplier {
        MaximumMultiplierBlockFullness::get()
    }
    fn target() -> Perquintill {
        TargetBlockFullness::get()
    }
    fn variability() -> Multiplier {
        AdjustmentVariableBlockFullness::get()
    }
}

pub struct BlobsLengthToFee<T: frame_system::Config>(core::marker::PhantomData<T>);

impl<T: frame_system::Config> sp_weights::WeightToFee for BlobsLengthToFee<T> {
    type Balance = Balance;

    fn weight_to_fee(weight: &Weight) -> Self::Balance {
        // really weird but weight.ref_time will contain the length of the extrinsic
        let length_fee = Self::Balance::saturated_from(weight.ref_time())
            .saturating_mul(TransactionByteFee::get());
        let multiplier = NextLengthMultiplier::get();

        // final adjusted length fee
        multiplier.saturating_mul_int(length_fee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MaxBlobSize, Runtime};
    use sp_runtime::BuildStorage;

    fn new_test_ext() -> sp_io::TestExternalities {
        frame_system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .unwrap()
            .into()
    }

    #[test]
    fn test_length_to_fee() {
        // Test that inclusion fee is evaluated propertly
        // following what done in BlobsLengthToFee
        new_test_ext().execute_with(|| {
            let len = 123;
            let multiplier = Multiplier::saturating_from_integer(12);
            NextLengthMultiplier::set(&multiplier);

            let length_fee = len * TransactionByteFee::get();
            let expected = multiplier.saturating_mul_int(length_fee);

            assert_eq!(
                pallet_transaction_payment::Pallet::<Runtime>::length_to_fee(len as u32),
                expected
            );
        });
    }

    #[test]
    fn test_blobs_fee_adjustment_convert() {
        use codec::Encode;
        use sp_core::twox_128;

        for len in (0..MaxBlobSize::get()).into_iter().step_by(100) {
            new_test_ext().execute_with(|| {
                // AllExtrinsicsLen is a private storage value of the system pallet
                // so the key must be manually constructed
                sp_io::storage::set(
                    &[twox_128(b"System"), twox_128(b"AllExtrinsicsLen")].concat(),
                    &len.encode(),
                );

                let fee_multiplier = Multiplier::saturating_from_rational(7, 8);

                let new_fee_multiplier = BlobsFeeAdjustment::<Runtime>::convert(fee_multiplier);

                // fee_multiplier should follow the standard behavior
                let expected_fee_multiplier = TargetedFeeAdjustment::<
                    Runtime,
                    TargetBlockFullness,
                    AdjustmentVariableBlockFullness,
                    MinimumMultiplierBlockFullness,
                    MaximumMultiplierBlockFullness,
                >::convert(fee_multiplier);
                assert_eq!(new_fee_multiplier, expected_fee_multiplier);

                // TODO: Ensure length multiplier is update properly
            });
        }
    }
}
