#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Currently, the `pallet_transaction_payment` uses the following formula:
///
/// ```ignore
/// inclusion_fee = base_fee + length_fee + [targeted_fee_adjustment * weight_fee];
/// ```
///
/// This formula allows us to update `targeted_fee_adjustment` at the end of each block
/// using `FeeMultiplierUpdate`, this associated type is called within the `on_finalize`
/// function of the `transaction_payment` pallet, with the purpose of converting the existing
/// `targeted_fee_adjustment` to a new one based on network congestion.
///
/// The goal of this pallet is to achieve a modified fee calculation formula:
///
/// ```ignore
/// inclusion_fee = base_fee + [targeted_length_fee_adjustment * length_fee] + [targeted_weight_fee_adjustment * weight_fee];
/// ```
///
/// `targeted_fee_adjustment` becomes `targeted_weight_fee_adjustment`,
/// while the behavior remains the same.
/// `targeted_length_fee_adjustment` is a new multiplier associate to `length_fee`.
/// This formula is achievable because the `transaction_payment`
/// pallet uses the `compute_fee_raw` function, which computes the final fee associated with an
/// extrinsic. This function utilizes the associated type `LengthToFee`, which converts the length
/// of an extrinsic to a fee.
///
/// By default, the implementation of `LengthToFee` is a constant multiplication. However, we
/// aim to achieve a dynamic formula, thanks to the new multiplier stored in `NextLenghtMultiplier`, implementing `sp_weights::WeightToFee`
/// for the Pallet struct and thus being able to use it as value for the associated type `LengthToFee` in the
/// `pallet_transaction_type::Config`.
///
/// `targeted_length_fee_adjustment` is updated at the end of each block inside `on_finalize`
#[frame_support::pallet]
pub mod pallet {

    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use pallet_transaction_payment::{Multiplier, OnChargeTransaction};
    use sp_runtime::{traits::Get, FixedPointNumber, Perquintill, SaturatedConversion, Saturating};

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_transaction_payment::Config {
        // `targeted_length_fee_adjustment` parameters
        #[pallet::constant]
        type TransactionByteFee: Get<<<Self as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<Self>>::Balance>;
        #[pallet::constant]
        type MaximumBlockLength: Get<u32>;
        #[pallet::constant]
        type AdjustmentVariableBlockSize: Get<Multiplier>;
        #[pallet::constant]
        type MinimumMultiplierBlockSize: Get<Multiplier>;
        #[pallet::constant]
        type MaximumMultiplierBlockSize: Get<Multiplier>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    pub struct NextLengthMultiplierDefualt;
    impl Get<Multiplier> for NextLengthMultiplierDefualt {
        fn get() -> Multiplier {
            Multiplier::saturating_from_integer(1)
        }
    }

    #[pallet::storage]
    pub type NextLengthMultiplier<T: Config> =
        StorageValue<_, Multiplier, ValueQuery, NextLengthMultiplierDefualt>;

    pub struct TargetBlockSizeDefault;
    impl Get<Perquintill> for TargetBlockSizeDefault {
        fn get() -> Perquintill {
            Perquintill::from_percent(16) // 0.8MiB
        }
    }

    #[pallet::storage]
    pub type TargetBlockSize<T: Config> =
        StorageValue<_, Perquintill, ValueQuery, TargetBlockSizeDefault>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_: BlockNumberFor<T>) -> Weight {
            // TODO: implement skip block logic
            // https://github.com/thrumdev/blobs/issues/165

            // NextLengthMultiplier: 1r + 1w
            // TargetBlockSize: 1r
            T::DbWeight::get().reads_writes(2, 1)
        }

        fn on_finalize(_n: BlockNumberFor<T>) {
            // update targeted_weight_fee_adjustment,
            // contained in NextLengthMultiplier storage item

            // This is essentially a copy-paste of the function TargetedFeeAdjustment::convert.
            // The main problem is that TargetedFeeAdjustment::convert directly calls the storage to extract the weight
            // of the current block, so there is no way to pass the length as an input argument and reuse the function to
            // update also the length multiplier.
            // Therefore, all the necessary parts taken and properly adapted to update NextLengthMultiplier.

            // Defensive only. The multiplier in storage should always be at most positive. Nonetheless
            // we recover here in case of errors, because any value below this would be stale and can
            // never change.
            let previous_len_multiplier = NextLengthMultiplier::<T>::get();
            let min_multiplier = T::MinimumMultiplierBlockSize::get();
            let max_multiplier = T::MaximumMultiplierBlockSize::get();
            let previous_len_multiplier = previous_len_multiplier.max(min_multiplier);

            // The limiting dimension is the length of all extrinsic
            let (normal_limiting_dimension, max_limiting_dimension) = (
                <frame_system::Pallet<T>>::all_extrinsics_len().min(T::MaximumBlockLength::get()),
                T::MaximumBlockLength::get() as u64,
            );

            let target_block_size = TargetBlockSize::<T>::get();
            let adjustment_variable = T::AdjustmentVariableBlockSize::get();

            let target_size = (target_block_size * max_limiting_dimension) as u128;
            let block_size = normal_limiting_dimension as u128;

            // determines if the first_term is positive
            let positive = block_size >= target_size;
            let diff_abs = block_size.max(target_size) - block_size.min(target_size);

            // defensive only, a test case assures that the maximum weight diff can fit in Multiplier
            // without any saturation.
            let diff =
                Multiplier::saturating_from_rational(diff_abs, max_limiting_dimension.max(1));
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

            NextLengthMultiplier::<T>::put(new_len_multiplier);
        }
    }

    impl<T: Config + pallet_transaction_payment::Config> sp_weights::WeightToFee for Pallet<T> {
        type Balance = <<T as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance;

        fn weight_to_fee(weight: &Weight) -> Self::Balance {
            // really weird but weight.ref_time will contain the length of the extrinsic
            let length_fee = Self::Balance::saturated_from(weight.ref_time())
                .saturating_mul(T::TransactionByteFee::get());
            let multiplier = NextLengthMultiplier::<T>::get();

            // final adjusted length fee
            multiplier.saturating_mul_int(length_fee)
        }
    }
}
