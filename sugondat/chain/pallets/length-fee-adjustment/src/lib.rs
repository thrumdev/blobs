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
///
/// The pallet also implements `cumulus_pallet_parachain_system::OnSystemEvent` which is used to update `NextLenghtMultiplier`
/// when blocks are skipped, the implementation follows the following formula to update the multiplier:
///
/// ```ignore
/// c_traffic = c_traffic * e^(-target*v*n)
/// ```
///
/// where the exponential is evaluated with the first SkippedBlocksNumberTerms terms, specified in the pallet Config, of the Taylor Series expansion of e^x.
#[frame_support::pallet]
pub mod pallet {

    use cumulus_pallet_parachain_system::OnSystemEvent;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use pallet_transaction_payment::{Multiplier, OnChargeTransaction};
    use polkadot_primitives::v6::{BlockNumber as RelayChainBlockNumber, PersistedValidationData};
    use sp_arithmetic::FixedU128;
    use sp_runtime::{
        traits::{Get, One, Zero},
        FixedPointNumber, Perquintill, SaturatedConversion, Saturating,
    };

    /// A provider for the last relay-chain block number, i.e. the relay-parent number of the
    /// previous block from _this_ chain.
    pub trait LastRelayBlockNumberProvider {
        /// Get the last relay chain block number.
        fn last_relay_block_number() -> RelayChainBlockNumber;
    }

    impl<T: cumulus_pallet_parachain_system::Config> LastRelayBlockNumberProvider for T {
        fn last_relay_block_number() -> RelayChainBlockNumber {
            cumulus_pallet_parachain_system::Pallet::<T>::last_relay_block_number()
        }
    }

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

        /// SkippedBlocksNumberTerms indicates the number of terms used in evaluating e^x in the Taylor expansion.
        /// The number of terms ensures that, given the following parameters, the error will be below a certain value.
        ///
        /// t = TargetBlockSize
        /// v = AdjustmentVariableBlockSize
        /// n = maximum number of skipped blocks
        ///
        /// (1 / (m+1)!) * (n * v * t)^(m + 1) <= err
        ///
        /// Here, m represents the number of terms needed to be below the error.
        /// The smallest value of m that satisfies the inequality should be used as the value for this parameter.
        #[pallet::constant]
        type SkippedBlocksNumberTerms: Get<u32>;

        /// The maximum number of skipped blocks is used to ensure that only the right amount of terms is used
        /// and no terms are wasted. If n is exceeded, the approximation could diverge significantly from the actual value of e^x.
        /// If the number of skipped blocks exceeds this value, it will be bounded to this value.
        /// However, this limitation comes at the cost of losing the real fee adjustment update.
        /// Therefore, producing a block at most every n skipped blocks should be enforced to avoid falling into this circumstance.
        #[pallet::constant]
        type MaximumSkippedBlocks: Get<u32>;

        /// A source to provide the relay-parent number of the previous block.
        type LastRelayBlockNumberProvider: LastRelayBlockNumberProvider;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    pub struct NextLengthMultiplierDefault;
    impl Get<Multiplier> for NextLengthMultiplierDefault {
        fn get() -> Multiplier {
            Multiplier::saturating_from_integer(1)
        }
    }

    #[pallet::storage]
    pub type NextLengthMultiplier<T: Config> =
        StorageValue<_, Multiplier, ValueQuery, NextLengthMultiplierDefault>;

    pub struct TargetBlockSizeDefault;
    impl Get<Perquintill> for TargetBlockSizeDefault {
        fn get() -> Perquintill {
            Perquintill::from_percent(16) // 0.8MiB
        }
    }

    #[pallet::storage]
    pub type TargetBlockSize<T: Config> =
        StorageValue<_, Perquintill, ValueQuery, TargetBlockSizeDefault>;

    /// Genesis config for setting up `NextLengthMultiplier` and `TargetBlockSize` storage values.
    /// Set None if the default values `NextLengthMultiplierDefault` & `TargetBlockSizeDefault` are to be used.
    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        /// Genesis set up for next length multiplier storage item
        pub next_length_multiplier: Option<Multiplier>,
        /// Genesis set up for target_block_size storage item
        pub target_block_size: Option<Perquintill>,
        _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            if let Some(next_len_mult) = self.next_length_multiplier {
                NextLengthMultiplier::<T>::put(next_len_mult)
            }
            if let Some(target_block_size) = self.target_block_size {
                TargetBlockSize::<T>::put(target_block_size)
            }
        }
    }

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

    impl<T: Config> OnSystemEvent for Pallet<T> {
        fn on_validation_data(data: &PersistedValidationData) {
            let relay_block_number = data.relay_parent_number;
            let prev_relay_block_number =
                T::LastRelayBlockNumberProvider::last_relay_block_number();

            // a value of zero here implies this is the first block of the parachain. no need
            // to do a massive fee update.
            if prev_relay_block_number == RelayChainBlockNumber::zero() {
                return;
            }

            // It should never be negative because the relay_block_number is surely
            // greater than the para_block_number.
            // However, saturating it to zero will prevent the multiplier from changing
            let relay_parent_distance = relay_block_number.saturating_sub(prev_relay_block_number);

            // TODO: update with `relay_parent_distance.saturating_sub(1)`
            // when updating to asynchronous backing
            // https://github.com/thrumdev/blobs/issues/166
            let n_skipped_blocks =
                (relay_parent_distance.saturating_sub(2) / 2).min(T::MaximumSkippedBlocks::get());

            let n_skipped_blocks = Multiplier::saturating_from_integer(n_skipped_blocks);
            let target_block_size = Multiplier::from(TargetBlockSize::<T>::get());
            let adjustment_variable = Multiplier::from(T::AdjustmentVariableBlockSize::get());

            let x = adjustment_variable
                .saturating_mul(target_block_size)
                .saturating_mul(n_skipped_blocks);

            // terms = sum_i (x^i / i!) where i in 0..=n_terms
            let mut terms = Multiplier::one().saturating_sub(x);

            let mut fact = Multiplier::one();
            let mut x_i = x;

            for index in 2..=T::SkippedBlocksNumberTerms::get() {
                x_i = x_i.saturating_mul(x);

                if x_i == Multiplier::zero() {
                    // If x_i is zero, the current term and all subsequent terms become useless,
                    // and therefore all next terms can be skipped.
                    // This happens when only a few blocks are skipped, making vnt very small
                    break;
                }

                fact = fact.saturating_mul(Multiplier::saturating_from_integer(index));

                // (-1)^index
                terms = match index & 1 {
                    1 => terms.saturating_sub(x_i.div(fact)),
                    _ => terms.saturating_add(x_i.div(fact)),
                };
            }

            NextLengthMultiplier::<T>::mutate(|multiplier| {
                *multiplier = multiplier.saturating_mul(terms);
            });
        }

        fn on_validation_code_applied() {}
    }
}
