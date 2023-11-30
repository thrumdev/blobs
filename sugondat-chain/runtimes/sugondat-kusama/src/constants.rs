/// Consensus-related.
pub mod consensus {
    use crate::BlockNumber;
    use frame_support::weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight};
    use sp_runtime::Perbill;

    /// Maximum number of blocks simultaneously accepted by the Runtime, not yet included
    /// into the relay chain.
    pub const UNINCLUDED_SEGMENT_CAPACITY: u32 = 1;
    /// How many parachain blocks are processed by the relay chain per parent. Limits the
    /// number of blocks authored per slot.
    pub const BLOCK_PROCESSING_VELOCITY: u32 = 1;
    /// Relay chain slot duration, in milliseconds.
    pub const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;

    /// This determines the average expected block time that we are targeting. Blocks will be
    /// produced at a minimum duration defined by `SLOT_DURATION`. `SLOT_DURATION` is picked up by
    /// `pallet_timestamp` which is in turn picked up by `pallet_aura` to implement `fn
    /// slot_duration()`.
    ///
    /// Change this to adjust the block time.
    pub const MILLISECS_PER_BLOCK: u64 = 12000;
    pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

    // Time is measured by number of blocks.
    pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;

    /// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
    /// used to limit the maximal weight of a single extrinsic.
    pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);
    /// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
    /// Operational  extrinsics.
    pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

    /// We allow for 0.5 seconds of compute with a 6 second average block time.
    pub const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
        WEIGHT_REF_TIME_PER_SECOND.saturating_div(2),
        cumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
    );
}

pub mod kusama {
    /// Constants relating to KSM.
    #[allow(unused)]
    pub mod currency {
        use crate::Balance;

        /// The existential deposit. Set to 1/10 of its parent Relay Chain.
        pub const EXISTENTIAL_DEPOSIT: Balance = 1 * CENTS / 10;

        pub const UNITS: Balance = 1_000_000_000_000;
        pub const QUID: Balance = UNITS / 30;
        pub const CENTS: Balance = QUID / 100;
        pub const GRAND: Balance = QUID * 1_000;
        pub const MILLICENTS: Balance = CENTS / 1_000;
    }

    /// Constants related to Kusama fee payment.
    pub mod fee {
        use crate::Balance;
        use frame_support::{
            pallet_prelude::Weight,
            weights::{
                constants::ExtrinsicBaseWeight, FeePolynomial, WeightToFeeCoefficient,
                WeightToFeeCoefficients, WeightToFeePolynomial,
            },
        };
        use smallvec::smallvec;
        pub use sp_runtime::Perbill;

        /// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
        /// node's balance type.
        ///
        /// This should typically create a mapping between the following ranges:
        ///   - [0, MAXIMUM_BLOCK_WEIGHT]
        ///   - [Balance::min, Balance::max]
        ///
        /// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
        ///   - Setting it to `0` will essentially disable the weight fee.
        ///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
        pub struct WeightToFee;
        impl frame_support::weights::WeightToFee for WeightToFee {
            type Balance = Balance;

            fn weight_to_fee(weight: &Weight) -> Self::Balance {
                let time_poly: FeePolynomial<Balance> = RefTimeToFee::polynomial().into();
                let proof_poly: FeePolynomial<Balance> = ProofSizeToFee::polynomial().into();

                // Take the maximum instead of the sum to charge by the more scarce resource.
                time_poly
                    .eval(weight.ref_time())
                    .max(proof_poly.eval(weight.proof_size()))
            }
        }

        /// Maps the reference time component of `Weight` to a fee.
        pub struct RefTimeToFee;
        impl WeightToFeePolynomial for RefTimeToFee {
            type Balance = Balance;
            fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
                // In Kusama, extrinsic base weight (smallest non-zero weight) is mapped to 1/10 CENT:
                // The standard system parachain configuration is 1/10 of that, as in 1/100 CENT.
                let p = super::currency::CENTS;
                let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());

                smallvec![WeightToFeeCoefficient {
                    degree: 1,
                    negative: false,
                    coeff_frac: Perbill::from_rational(p % q, q),
                    coeff_integer: p / q,
                }]
            }
        }

        /// Maps the proof size component of `Weight` to a fee.
        pub struct ProofSizeToFee;
        impl WeightToFeePolynomial for ProofSizeToFee {
            type Balance = Balance;
            fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
                // Map 10kb proof to 1 CENT.
                let p = super::currency::CENTS;
                let q = 10_000;

                smallvec![WeightToFeeCoefficient {
                    degree: 1,
                    negative: false,
                    coeff_frac: Perbill::from_rational(p % q, q),
                    coeff_integer: p / q,
                }]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // compile-test: ensure that the balance type used by Kusama matches the one used here.
    fn balance_type_matches() {
        assert_eq!(
            std::any::TypeId::of::<crate::Balance>(),
            std::any::TypeId::of::<polkadot_core_primitives::Balance>(),
        );
    }
}
