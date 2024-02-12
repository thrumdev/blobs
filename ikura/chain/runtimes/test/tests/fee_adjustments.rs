use frame_support::traits::Hooks;
use frame_support::{
    dispatch::DispatchClass,
    weights::{Weight, WeightToFee},
};
use ikura_primitives::MAXIMUM_BLOCK_LENGTH;
use ikura_test_runtime::{
    AdjustmentVariableBlockFullness, AdjustmentVariableBlockSize, LengthFeeAdjustment,
    MinimumMultiplierBlockFullness, MinimumMultiplierBlockSize, Runtime,
    RuntimeBlockWeights as BlockWeights, SlowAdjustingFeeUpdate, System, TargetBlockFullness,
    TransactionPayment, CENTS, DAYS, MILLICENTS,
};
use pallet_ikura_length_fee_adjustment::{NextLengthMultiplier, TargetBlockSize};
use pallet_transaction_payment::Multiplier;
use sp_runtime::{
    assert_eq_error_rate,
    traits::{Convert, One, Zero},
    BuildStorage, FixedPointNumber,
};

// The following tests check if the parameters used to define
// targeted_length_fee_adjustment and targeted_weight_fee_adjustment
// in the formula
//
// inclusion_fee = base_fee + [targeted_length_fee_adjustment * length_fee] + [targeted_weight_fee_adjustment * weight_fee];
//
// are updated correctly, do not overflow and can recover from zero
#[derive(Clone, Copy)]
enum MultiplierType {
    Length,
    Fee,
}

impl MultiplierType {
    fn max(&self) -> Weight {
        match self {
            MultiplierType::Length => Weight::from_parts(MAXIMUM_BLOCK_LENGTH as u64, 0),
            MultiplierType::Fee => BlockWeights::get()
                .get(DispatchClass::Normal)
                .max_total
                .unwrap_or_else(|| BlockWeights::get().max_block),
        }
    }

    fn min_multiplier(self: MultiplierType) -> Multiplier {
        match self {
            MultiplierType::Length => MinimumMultiplierBlockSize::get(),
            MultiplierType::Fee => MinimumMultiplierBlockFullness::get(),
        }
    }

    fn target(self: MultiplierType) -> Weight {
        let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .unwrap()
            .into();

        t.execute_with(|| match self {
            MultiplierType::Length => TargetBlockSize::<Runtime>::get() * self.max(),
            MultiplierType::Fee => TargetBlockFullness::get() * self.max(),
        })
    }

    // update based on runtime impl.
    fn runtime_multiplier_update(&self, fm: Multiplier) -> Multiplier {
        match self {
            MultiplierType::Fee => SlowAdjustingFeeUpdate::<Runtime>::convert(fm),
            MultiplierType::Length => {
                // the previous multiplier is fetched from the storage
                NextLengthMultiplier::<Runtime>::put(fm);
                LengthFeeAdjustment::on_finalize(0);
                NextLengthMultiplier::<Runtime>::get()
            }
        }
    }

    // update based on reference impl.
    fn truth_value_update(&self, block_weight: Weight, previous: Multiplier) -> Multiplier {
        let accuracy = Multiplier::accuracy() as f64;
        let previous_float = previous.into_inner() as f64 / accuracy;
        // bump if it is zero.
        let previous_float =
            previous_float.max(self.min_multiplier().into_inner() as f64 / accuracy);

        let max = self.max();
        let target_weight = self.target();
        // when self == MultiplierType::Length
        // the proof_size is always zero so the ref_time will be treated as Length
        let normalized_weight_dimensions = (
            block_weight.ref_time() as f64 / max.ref_time() as f64,
            block_weight.proof_size() as f64 / max.proof_size() as f64,
        );

        let (normal, max, target) =
            if normalized_weight_dimensions.0 < normalized_weight_dimensions.1 {
                (
                    block_weight.proof_size(),
                    max.proof_size(),
                    target_weight.proof_size(),
                )
            } else {
                (
                    block_weight.ref_time(),
                    max.ref_time(),
                    target_weight.ref_time(),
                )
            };

        // maximum tx weight
        let m = max as f64;
        // block weight always truncated to max
        let block_weight = (normal as f64).min(m);

        let v: f64 = self.adjustment_variable().to_float();

        // ideal saturation in terms of weight
        let ss = target as f64;
        // current saturation in terms of weight
        let s = block_weight;

        let t1 = v * (s / m - ss / m);
        let t2 = v.powi(2) * (s / m - ss / m).powi(2) / 2.0;
        let next_float = previous_float * (1.0 + t1 + t2);
        Multiplier::from_float(next_float)
    }

    fn adjustment_variable(&self) -> Multiplier {
        match self {
            MultiplierType::Length => AdjustmentVariableBlockSize::get(),
            MultiplierType::Fee => AdjustmentVariableBlockFullness::get(),
        }
    }

    fn target_percentage(&self) -> Multiplier {
        match self {
            MultiplierType::Length => TargetBlockSize::<Runtime>::get().into(),
            MultiplierType::Fee => TargetBlockFullness::get().into(),
        }
    }

    fn run_with<F>(&self, w: Weight, assertions: F)
    where
        F: Fn() -> (),
    {
        let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .unwrap()
            .into();
        t.execute_with(|| {
            // resources.0 = BlockWeight, resources.1 = AllExtrinsicsLen
            let resources = match self {
                MultiplierType::Length => (Weight::zero(), w.ref_time() as usize),
                MultiplierType::Fee => (w, 0),
            };
            System::set_block_consumed_resources(resources.0, resources.1);

            assertions()
        });
    }
}

#[test]
fn truth_value_update_poc_works() {
    let fm = Multiplier::saturating_from_rational(1, 2);

    let test = |mul_type: MultiplierType| {
        let test_set = vec![
            (Weight::zero(), fm),
            (Weight::from_parts(100, 0), fm),
            (Weight::from_parts(1000, 0), fm),
            (mul_type.target(), fm),
            (mul_type.max() / 2, fm),
            (mul_type.max(), fm),
        ];

        test_set.into_iter().for_each(|(w, fm)| {
            mul_type.run_with(w, || {
                assert_eq_error_rate!(
                    mul_type.truth_value_update(w, fm),
                    mul_type.runtime_multiplier_update(fm),
                    Multiplier::from_inner(100)
                );
            })
        });
    };

    test(MultiplierType::Fee);
    test(MultiplierType::Length);
}

#[test]
fn multiplier_can_grow_from_zero() {
    // if the min is too small, then this will not change, and we are doomed forever.
    let test = |mul_type: MultiplierType, weight: Weight| {
        mul_type.run_with(weight, || {
            let next = mul_type.runtime_multiplier_update(mul_type.min_multiplier());
            assert!(
                next > mul_type.min_multiplier(),
                "{:?} !> {:?}",
                next,
                mul_type.min_multiplier()
            );
        });
    };

    // the block ref time is 1/100th bigger than target.
    test(
        MultiplierType::Fee,
        Weight::from_parts(MultiplierType::Fee.target().ref_time() * 101 / 100, 0),
    );
    // the block proof size is 1/100th bigger than target.
    test(
        MultiplierType::Fee,
        Weight::from_parts(0, (MultiplierType::Fee.target().proof_size() / 100) * 101),
    );
    // the block length is 1/100th bigger than target.
    test(
        MultiplierType::Length,
        Weight::from_parts(MultiplierType::Length.target().ref_time() * 101 / 100, 0),
    );
}

#[test]
fn multiplier_cannot_go_below_limit() {
    // will not go any further below even if block is empty.
    let test = |mul_type: MultiplierType| {
        mul_type.run_with(Weight::zero(), || {
            let next = mul_type.runtime_multiplier_update(mul_type.min_multiplier());
            assert_eq!(next, mul_type.min_multiplier());
        })
    };

    test(MultiplierType::Fee);
    test(MultiplierType::Length);
}

#[test]
fn time_to_reach_zero() {
    // TODO: 1440 needs to be used instead of 7200
    // when updating to asynchronous backing
    // https://github.com/thrumdev/blobs/issues/166
    //
    // blocks per 24h in cumulus-node: 7200 (k)
    // s* = 0.1875 (TargetBlockFullness) or 0.16 (TargetBlockSize)
    // The bound from the research in an empty chain is:
    // v <~ (p / k(0 - s*))
    // p > v * k * -s*
    // to get p == -1 we'd need
    // -1 > v * k * -s*
    // 1 < v * k * s*
    // 1 / (v * s*) < k
    //
    // if s* = 0.1875
    //  then k > 71_111 ~ 9.8 days
    // else s* = 0.16
    //  then k > 83_333 ~ 11.5 days

    let test = |mul_type: MultiplierType| {
        mul_type.run_with(Weight::zero(), || {
            // start from 1, the default.
            let mut fm = Multiplier::one();
            let mut iterations: u64 = 0;
            let limit = (Multiplier::saturating_from_integer(1)
                / (mul_type.target_percentage() * mul_type.adjustment_variable()))
            .to_float()
            .ceil() as u64;

            loop {
                let next = mul_type.runtime_multiplier_update(fm);
                fm = next;
                if fm <= mul_type.min_multiplier() {
                    break;
                }
                iterations += 1;
            }
            assert!(iterations > limit);
        })
    };

    test(MultiplierType::Fee);
    test(MultiplierType::Length);
}

#[test]
fn time_to_reach_one() {
    // TODO: 1440 needs to ne used insted of 7200
    // when updating to asynchronous backing
    // https://github.com/thrumdev/blobs/issues/166
    //
    // blocks per 24h in cumulus-node: 7200 (k)
    // s* = 0.1875 (TargetBlockFullness) or 0.16 (TargetBlockSize)
    // The bound from the research in an full chain is:
    // v <~ (p / k(1 - s*))
    // p > v * k * (1 - s*)
    // to get p == 1 we'd need (going from zero to target)
    // 1 > v * k * (1 - s*)
    // k < 1 / (v * (1 - s*))

    // if s* = 0.1875
    //  then k > 17_778 ~ 2.47 days
    // else s* = 0.16
    //  then k > 1000 ~ 3.3 hours

    let test = |mul_type: MultiplierType| {
        mul_type.run_with(mul_type.max(), || {
            // start from min_multiplier, the default.
            let mut fm = mul_type.min_multiplier();
            let mut iterations: u64 = 0;

            let limit = (Multiplier::saturating_from_integer(1)
                / ((Multiplier::saturating_from_integer(1) - mul_type.target_percentage())
                    * mul_type.adjustment_variable()))
            .to_float()
            .ceil() as u64;

            loop {
                let next = mul_type.runtime_multiplier_update(fm);
                fm = next;
                if fm >= Multiplier::one() {
                    break;
                }
                iterations += 1;
            }
            assert!(iterations > limit);
        })
    };

    test(MultiplierType::Fee);
    test(MultiplierType::Length);
}

#[test]
fn min_change_per_day() {
    let test = |mul_type: MultiplierType| {
        mul_type.run_with(mul_type.max(), || {
            let mut fm = Multiplier::one();
            // See the example in the doc of `TargetedFeeAdjustment`. are at least 0.234, hence
            // `fm > 1.234`.
            for _ in 0..DAYS {
                let next = mul_type.runtime_multiplier_update(fm);
                fm = next;
            }
            // TODO: 1440 needs to ne used insted of 7200
            // when updating to asynchronous backing
            // https://github.com/thrumdev/blobs/issues/166
            //
            // 7200 blocks per day with one 12 seconds blocks
            // v * k * (1 - s)
            let expected = mul_type.adjustment_variable()
                * Multiplier::saturating_from_integer(7200)
                * (Multiplier::saturating_from_integer(1) - mul_type.target_percentage());
            assert!(fm > expected);
        })
    };
    test(MultiplierType::Fee);
    test(MultiplierType::Length);
}

#[test]
#[ignore]
fn congested_chain_fee_simulation() {
    // `cargo test congested_chain_simulation -- --nocapture` to get some insight.

    // almost full. The entire quota of normal transactions is taken.
    let block_weight = BlockWeights::get()
        .get(DispatchClass::Normal)
        .max_total
        .unwrap()
        - Weight::from_parts(100, 0);

    // Default substrate weight.
    let tx_weight = frame_support::weights::constants::ExtrinsicBaseWeight::get();

    MultiplierType::Fee.run_with(block_weight, || {
        // initial value configured on module
        let mut fm = Multiplier::one();
        assert_eq!(fm, TransactionPayment::next_fee_multiplier());

        let mut iterations: u64 = 0;
        loop {
            let next = MultiplierType::Fee.runtime_multiplier_update(fm);
            // if no change, panic. This should never happen in this case.
            if fm == next {
                panic!("The fee should ever increase");
            }
            fm = next;
            iterations += 1;
            let fee = <Runtime as pallet_transaction_payment::Config>::WeightToFee::weight_to_fee(
                &tx_weight,
            );
            let adjusted_fee = fm.saturating_mul_acc_int(fee);
            println!(
                "iteration {}, new fm = {:?}. Fee at this point is: {} units / {} millicents, \
                    {} cents",
                iterations,
                fm,
                adjusted_fee,
                adjusted_fee / MILLICENTS,
                adjusted_fee / CENTS
            );
        }
    });
}

#[test]
#[ignore]
fn congested_chain_length_simulation() {
    // almost full
    let block_length = Weight::from_parts(MAXIMUM_BLOCK_LENGTH as u64 - 100, 0);

    // Default substrate weight.
    let tx_weight = frame_support::weights::constants::ExtrinsicBaseWeight::get();

    MultiplierType::Length.run_with(block_length, || {
        // initial value configured on module
        let mut fm = Multiplier::one();
        assert_eq!(fm, NextLengthMultiplier::<Runtime>::get());

        let mut iterations: u64 = 0;
        loop {
            let next = MultiplierType::Length.runtime_multiplier_update(fm);
            // if no change, panic. This should never happen in this case.
            if fm == next {
                panic!("The fee should ever increase");
            }
            fm = next;
            iterations += 1;
            let fee = <Runtime as pallet_transaction_payment::Config>::WeightToFee::weight_to_fee(
                &tx_weight,
            );
            let adjusted_fee = fm.saturating_mul_acc_int(fee);
            println!(
                "iteration {}, new fm = {:?}. Fee at this point is: {} units / {} millicents, \
                    {} cents",
                iterations,
                fm,
                adjusted_fee,
                adjusted_fee / MILLICENTS,
                adjusted_fee / CENTS
            );
        }
    });
}

#[test]
fn stateless_weight_mul() {
    let test = |mul_type: MultiplierType| {
        let fm = Multiplier::saturating_from_rational(1, 2);
        mul_type.run_with(mul_type.target() / 4, || {
            let next = mul_type.runtime_multiplier_update(fm);
            assert_eq_error_rate!(
                next,
                mul_type.truth_value_update(mul_type.target() / 4, fm),
                Multiplier::from_inner(100)
            );

            // Light block. Multiplier is reduced a little.
            assert!(next < fm);
        });

        mul_type.run_with(mul_type.target() / 2, || {
            let next = mul_type.runtime_multiplier_update(fm);
            assert_eq_error_rate!(
                next,
                mul_type.truth_value_update(mul_type.target() / 2, fm),
                Multiplier::from_inner(100)
            );
            // Light block. Multiplier is reduced a little.
            assert!(next < fm);
        });
        mul_type.run_with(mul_type.target(), || {
            let next = mul_type.runtime_multiplier_update(fm);
            assert_eq_error_rate!(
                next,
                mul_type.truth_value_update(mul_type.target(), fm),
                Multiplier::from_inner(100)
            );
            // ideal. No changes.
            assert_eq!(next, fm)
        });
        mul_type.run_with(mul_type.target() * 2, || {
            // More than ideal. Fee is increased.
            let next = mul_type.runtime_multiplier_update(fm);
            assert_eq_error_rate!(
                next,
                mul_type.truth_value_update(mul_type.target() * 2, fm),
                Multiplier::from_inner(100)
            );

            // Heavy block. Fee is increased a little.
            assert!(next > fm);
        });
    };

    test(MultiplierType::Fee);
    test(MultiplierType::Length);
}

#[test]
fn weight_mul_grow_on_big_block() {
    let test = |mul_type: MultiplierType| {
        mul_type.run_with(mul_type.target() * 2, || {
            let mut original = Multiplier::zero();
            let mut next = Multiplier::default();

            (0..1_000).for_each(|_| {
                next = mul_type.runtime_multiplier_update(original);
                assert_eq_error_rate!(
                    next,
                    mul_type.truth_value_update(mul_type.target() * 2, original),
                    Multiplier::from_inner(100)
                );
                // must always increase
                assert!(next > original, "{:?} !>= {:?}", next, original);
                original = next;
            });
        });
    };

    test(MultiplierType::Fee);
    test(MultiplierType::Length);
}

#[test]
fn weight_mul_decrease_on_small_block() {
    let test = |mul_type: MultiplierType| {
        mul_type.run_with(mul_type.target() / 2, || {
            let mut original = Multiplier::saturating_from_rational(1, 2);
            let mut next;

            for _ in 0..100 {
                // decreases
                next = mul_type.runtime_multiplier_update(original);
                assert!(next < original, "{:?} !<= {:?}", next, original);
                original = next;
            }
        })
    };

    test(MultiplierType::Fee);
    test(MultiplierType::Length);
}

#[test]
fn weight_to_fee_should_not_overflow_on_large_weights() {
    let kb_time = Weight::from_parts(1024, 0);
    let kb_size = Weight::from_parts(0, 1024);
    let mb_time = 1024u64 * kb_time;
    let max_fm = Multiplier::saturating_from_integer(i128::MAX);

    let test_weights = |mul_type: MultiplierType, weights: Vec<Weight>| {
        weights.into_iter().for_each(|i| {
            mul_type.run_with(i, || {
                let next = mul_type.runtime_multiplier_update(Multiplier::one());
                let truth = mul_type.truth_value_update(i, Multiplier::one());
                assert_eq_error_rate!(truth, next, Multiplier::from_inner(50_000_000));
            });
        });
    };

    // check that for all values it can compute, correctly.
    let ref_time_and_length_tests = vec![
        Weight::zero(),
        // testcases ignoring proof size part of the weight.
        Weight::from_parts(1, 0),
        Weight::from_parts(10, 0),
        Weight::from_parts(1000, 0),
        kb_time,
        10u64 * kb_time,
        100u64 * kb_time,
        mb_time,
        10u64 * mb_time,
        Weight::from_parts(2147483647, 0),
        Weight::from_parts(4294967295, 0),
    ];

    let proof_size_tests = vec![
        // testcases ignoring ref time part of the weight.
        Weight::from_parts(0, 100000000000),
        1000000u64 * kb_size,
        1000000000u64 * kb_size,
        Weight::from_parts(0, 18014398509481983),
        Weight::from_parts(0, 9223372036854775807),
    ];

    let ref_time_and_proof_size_tests = vec![
        // test cases with both parts of the weight.
        BlockWeights::get().max_block / 1024,
        BlockWeights::get().max_block / 2,
        BlockWeights::get().max_block,
        Weight::MAX / 2,
        Weight::MAX,
    ];

    test_weights(MultiplierType::Fee, ref_time_and_length_tests.clone());
    test_weights(MultiplierType::Fee, proof_size_tests);
    test_weights(MultiplierType::Fee, ref_time_and_proof_size_tests);
    test_weights(MultiplierType::Length, ref_time_and_length_tests);

    // Some values that are all above the target and will cause an increase.
    let test = |mul_type: MultiplierType| {
        let t = mul_type.target();
        vec![
            t + Weight::from_parts(100, 0),
            // this is the same as before for MultiplierType::Length
            t + Weight::from_parts(0, t.proof_size() * 2),
            t * 2,
            t * 4,
        ]
        .into_iter()
        .for_each(|i| {
            mul_type.run_with(i, || {
                let fm = mul_type.runtime_multiplier_update(max_fm);
                // won't grow. The convert saturates everything.
                assert_eq!(fm, max_fm);
            })
        });
    };

    test(MultiplierType::Fee);
    test(MultiplierType::Length);
}
