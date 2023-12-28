#![cfg_attr(not(feature = "std"), no_std)]
use pallet_transaction_payment::{Multiplier, MultiplierUpdate, TargetedFeeAdjustment};
use sp_arithmetic::traits::{BaseArithmetic, Unsigned};
use sp_runtime::{
    traits::{Bounded, Convert, Get},
    FixedPointNumber, Perquintill, SaturatedConversion, Saturating,
};
use sp_weights::Weight;

frame_support::parameter_types! {
    pub storage NextLengthMultiplier: Multiplier = Multiplier::saturating_from_integer(1);
}

/// Currently, the `pallet_transaction_payment` uses the following formula:
///
/// ```ignore
/// inclusion_fee = base_fee + length_fee + [targeted_fee_adjustment * weight_fee];
/// ```
///
/// This formula allows us to update `targeted_fee_adjustment` at the end of each block
/// using `FeeMultiplierUpdate`. This associated type is called within the `on_finalize`
/// function of the `transaction_payment` pallet, with the purpose of converting the existing
/// `targeted_fee_adjustment` to a new one based on network congestion.
///
/// The goal of this struct is to achieve a modified fee calculation formula:
///
/// ```ignore
/// inclusion_fee = base_fee + [targeted_length_fee_adjustment * length_fee] + [targeted_weight_fee_adjustment * weight_fee];
/// ```
///
/// As you can see, `targeted_fee_adjustment` becomes `targeted_weight_fee_adjustment`,
/// while the behavior remains the same. The side effect is adding the multiplier
/// `targeted_length_fee_adjustment` to `length_fee`. This formula is achievable because the `transaction_payment`
/// pallet uses the `compute_fee_raw` function, which computes the final fee associated with an
/// extrinsic. This function utilizes the associated type `LengthToFee`, which converts the length
/// of an extrinsic to a fee.
///
/// By default, the implementation of `LengthToFee` is a constant multiplication. However, we
/// aim to achieve a dynamic formula that can adapt based on network usage. This requires a custom
/// implementation of `LengthToFee` in addition to this struct, called `DynamicLengthToFee`.
///
/// This struct solely provides a dynamic update of `targeted_length_fee_adjustment` and
/// `targeted_weight_fee_adjustment` based on block congestion and usage. The effective implementation
/// of the formula described above is made possible by using `DynamicLengthToFee`.
pub struct DynamicFeeAdjustment<
    T,
    TF,   // TargetBlockFullness
    AF,   //AdjustmentVariableBlockFullness
    MF,   //MinimumMultiplierBlockFullness
    MaF,  //MaximumMultiplierBlockFullness
    MBLS, //MaximumBlockLength
    TS,   // TargetBlockSize
    AS,   //AdjustmentVariableBlockSize,
    MS,   //MinimumMultiplierBlockSize,
    MaS,  //MaximumMultiplierBlockSize,
>(core::marker::PhantomData<(T, TF, AF, MF, MaF, MBLS, TS, AS, MS, MaS)>);

impl<T, TF, AF, MF, MaF, MBLS, TS, AS, MS, MaS> Convert<Multiplier, Multiplier>
    for DynamicFeeAdjustment<T, TF, AF, MF, MaF, MBLS, TS, AS, MS, MaS>
where
    T: frame_system::Config,
    TF: Get<Perquintill>,
    AF: Get<Multiplier>,
    MF: Get<Multiplier>,
    MaF: Get<Multiplier>,
    MBLS: Get<u32>,
    TS: Get<Perquintill>,
    AS: Get<Multiplier>,
    MS: Get<Multiplier>,
    MaS: Get<Multiplier>,
{
    // This function should be a pure function used to update NextFeeMultiplier
    // but will also have the side effect of updating NextLengthMultiplier.
    fn convert(previous_fee_multiplier: Multiplier) -> Multiplier {
        // Update NextLengthMultiplier

        // To update the value, the same formula as TargetedFeeAdjustment will be used.
        // The formula is described here:
        // https://research.web3.foundation/Polkadot/overview/token-economics#2-slow-adjusting-mechanism

        // This is essentially a copy-paste of that function because it works with normalized measurements.
        // Therefore, the multipliers will be properly evaluated for ref_time, proof_size, and length of the extrinsic.

        // The main problem is that TargetedFeeAdjustment::convert directly calls the storage to extract the weight
        // of the current block, so there is no way to pass the length as an input argument and reuse the function to
        // update also the length multiplier.
        // Therefore, all the necessary parts will be copied and pasted to properly update NextLengthMultiplier.

        // Defensive only. The multiplier in storage should always be at most positive. Nonetheless
        // we recover here in case of errors, because any value below this would be stale and can
        // never change.
        let previous_len_multiplier = NextLengthMultiplier::get();
        let min_multiplier = MS::get();
        let max_multiplier = MaS::get();
        let previous_len_multiplier = previous_len_multiplier.max(min_multiplier);

        // The limiting dimension is the length of all extrinsic
        let (normal_limiting_dimension, max_limiting_dimension) = (
            <frame_system::Pallet<T>>::all_extrinsics_len().min(MBLS::get()),
            MBLS::get() as u64,
        );

        let target_block_size = TS::get();
        let adjustment_variable = AS::get();

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
        // Here is the tricky part: this method returns the new value associated with the old-fashioned `NextFeeMultiplier`,
        // because weight dynamic adjustment has been battle tested. Previously, we have updated the
        // `NextLengthMultiplier` used in `DynamicLengthToFee`.
        TargetedFeeAdjustment::<T, TF, AF, MF, MaF>::convert(previous_fee_multiplier)
    }
}

impl<T, TF, AF, MF, MaF, MBLS, TS, AS, MS, MaS> MultiplierUpdate
    for DynamicFeeAdjustment<T, TF, AF, MF, MaF, MBLS, TS, AS, MS, MaS>
where
    T: frame_system::Config,
    TF: Get<Perquintill>,
    AF: Get<Multiplier>,
    MF: Get<Multiplier>,
    MaF: Get<Multiplier>,
    MBLS: Get<u32>,
    TS: Get<Perquintill>,
    AS: Get<Multiplier>,
    MS: Get<Multiplier>,
    MaS: Get<Multiplier>,
{
    fn min() -> Multiplier {
        MF::get()
    }
    fn max() -> Multiplier {
        MaF::get()
    }
    fn target() -> Perquintill {
        TF::get()
    }
    fn variability() -> Multiplier {
        AF::get()
    }
}

pub struct DynamicLengthToFee<T, B, M>(core::marker::PhantomData<(T, B, M)>);

impl<T, B, M> sp_weights::WeightToFee for DynamicLengthToFee<T, B, M>
where
    T: frame_system::Config,
    B: BaseArithmetic + From<u32> + Copy + Unsigned,
    M: Get<B>,
{
    type Balance = B;

    fn weight_to_fee(weight: &Weight) -> Self::Balance {
        // really weird but weight.ref_time will contain the length of the extrinsic
        let length_fee = Self::Balance::saturated_from(weight.ref_time()).saturating_mul(M::get());
        let multiplier = NextLengthMultiplier::get();

        // final adjusted length fee
        multiplier.saturating_mul_int(length_fee)
    }
}

/*
#[cf//g(test)]
mod //tests {
    //use super::*;
    //use crate::Runtime;
    //use sp_runtime::BuildStorage;

    //fn new_test_ext() -> sp_io::TestExternalities {
    //    frame_system::GenesisConfig::<Runtime>::default()
    //        .build_storage()
    //        .unwrap()
    //        .into()
    //}

    //#[test]
    //fn test_length_to_fee() {
    //    // Test that inclusion fee is evaluated propertly
    //    // following what done in BlobsLengthToFee
    //    new_test_ext().execute_with(|| {
    //        let len = 123;
    //        let multiplier = Multiplier::saturating_from_integer(12);
    //        NextLengthMultiplier::set(&multiplier);

    //        let length_fee = len * TransactionByteFee::get();
    //        let expected = multiplier.saturating_mul_int(length_fee);

    //        assert_eq!(
    //            pallet_transaction_payment::Pallet::<Runtime>::length_to_fee(len as u32),
    //            expected
    //        );
    //    });
    //}
}
*/
