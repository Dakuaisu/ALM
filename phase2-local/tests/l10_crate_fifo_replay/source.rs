// Frozen HTML 4060–4073 advances `_extra` only after both component cost calls.
// The report multiplies the rounded per-crate cost (3048–3052); moneyR (394–401) rounds again for display.
pub(super) fn assert_cost(packets: u32, expected: [f64; 4]) {
    let second_packets = f64::from(packets);
    let wrong_components = [3.75, second_packets * 1.25];
    let sequential_components = [3.75, second_packets * 2.75];
    let frozen_unit = rounded_crate_unit(wrong_components);
    let separately_rounded = sequential_components
        .map(|cost| round(cost / 2.0 * 100.0))
        .into_iter()
        .sum::<f64>()
        * 2.0;
    let actual = [
        frozen_unit * 2.0 * 100.0,
        rounded_crate_unit(sequential_components) * 2.0 * 100.0,
        separately_rounded,
        round(frozen_unit * 2.0),
    ];
    for (value, fixed) in actual.into_iter().zip(expected) {
        assert!(
            (value - fixed).abs() < 1e-9,
            "expected {expected:?}; got {actual:?}"
        );
    }
    println!(
        "L10-01 source differential [3,{packets}]: wrong oldest reused raw component rupees={wrong_components:?}, stored cost per crate=₹{frozen_unit}; fixed={expected:?}, calculated={actual:?} (legacy saved minor, sequential rounded minor, separately rounded minor, legacy displayed whole rupees)"
    );
}

fn rounded_crate_unit(costs: [f64; 2]) -> f64 {
    round(costs.into_iter().sum::<f64>() / 2.0 * 100.0) / 100.0
}

fn round(nonnegative: f64) -> f64 {
    (nonnegative + 0.5).floor()
}

pub(super) fn assert_unrepresentable_inputs() {
    let fractional_packets = 2.0 / 4.0;
    let fractional_minor = fractional_packets * 125.0;
    let subminor_rate = 1.125 * 100.0;
    assert!((fractional_minor - 62.5_f64).abs() < 1e-9);
    assert!((subminor_rate - 112.5_f64).abs() < 1e-9);
    println!(
        "L10-01 source-only limits: each half-packet demand costs {fractional_minor} minor; Rust rejects it even if both sum to one packet. A source receipt rate of ₹1.125 ({subminor_rate} minor) cannot be an integer Money input."
    );
}
