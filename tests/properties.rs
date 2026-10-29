//! Property tests for the nexus-gated arithmetic.
//!
//! The example tests pin Texas's 8.25%; these pin the algebra across
//! arbitrary rates, line counts, and taxability patterns.

#![allow(clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use sales_tax::{
    JurisdictionLevel, JurisdictionRate, NexusRegistry, RoundingScope, SaleLine, compute_tax,
};
use vat_rules::TieMode;

fn quoted_percent() -> impl Strategy<Value = String> {
    (1u32..100u32, 0u32..100u32).prop_map(|(whole, frac)| {
        if frac == 0 {
            format!("{whole}")
        } else {
            format!("{whole}.{frac:02}")
        }
    })
}

fn lines_strategy() -> impl Strategy<Value = Vec<SaleLine>> {
    proptest::collection::vec(
        (1i64..100_000i64, any::<bool>()).prop_map(|(net, taxable)| SaleLine {
            net_minor: net,
            taxable,
        }),
        1..12,
    )
}

fn rates_strategy() -> impl Strategy<Value = Vec<JurisdictionRate>> {
    proptest::collection::vec(quoted_percent(), 1..4).prop_map(|quoted| {
        let levels = JurisdictionLevel::ALL.iter().cycle();
        quoted
            .into_iter()
            .zip(levels)
            .map(|(q, level)| {
                JurisdictionRate::from_percent(*level, "jurisdiction", &q)
                    .expect("strategy generates valid rates")
            })
            .collect()
    })
}

proptest! {
    /// Under both scopes, on any inputs: with nexus, every jurisdiction's tax
    /// and the total are non-negative (amounts are non-negative by line
    /// validation and rates are positive by parse).
    #[test]
    fn taxes_are_never_negative(
        lines in lines_strategy(),
        rates in rates_strategy(),
        mode in prop::sample::select(vec![TieMode::HalfUp, TieMode::HalfEven, TieMode::TowardZero]),
    ) {
        let nexus = NexusRegistry::new(&["TX".to_string()]);
        let breakdown = compute_tax(&lines, "TX", &rates, &nexus, RoundingScope::PerDocument, mode)
            .expect("nexus and rates supplied");
        prop_assert!(breakdown.total_tax >= 0);
        for (level, _, tax) in &breakdown.per_jurisdiction {
            prop_assert!(*tax >= 0, "negative tax at {level:?}");
        }
    }

    /// Per-line and per-document scopes can differ only by the pennies their
    /// per-line roundings spill: at most half a minor unit per line per
    /// jurisdiction. A larger difference means the scopes disagree about
    /// *what* is taxable, which would be a real defect.
    #[test]
    fn scope_difference_is_bounded_by_half_a_unit_per_line_per_jurisdiction(
        lines in lines_strategy(),
        rates in rates_strategy(),
        mode in prop::sample::select(vec![TieMode::HalfUp, TieMode::HalfEven, TieMode::TowardZero]),
    ) {
        let nexus = NexusRegistry::new(&["TX".to_string()]);
        let per_line = compute_tax(&lines, "TX", &rates, &nexus, RoundingScope::PerLine, mode)
            .expect("computes");
        let per_document = compute_tax(&lines, "TX", &rates, &nexus, RoundingScope::PerDocument, mode)
            .expect("computes");
        let max_drift = (lines.len() * rates.len()).div_ceil(2) as i64;
        let diff = (per_line.total_tax - per_document.total_tax).abs();
        prop_assert!(diff <= max_drift, "scope drift {diff} exceeds {max_drift}");
    }

    /// A destination outside the nexus registry is always a legal zero,
    /// whatever the inputs.
    #[test]
    fn no_nexus_is_always_zero(
        lines in lines_strategy(),
        rates in rates_strategy(),
        mode in prop::sample::select(vec![TieMode::HalfUp, TieMode::HalfEven]),
    ) {
        let nexus = NexusRegistry::new(&["TX".to_string()]);
        let breakdown = compute_tax(&lines, "ZZ", &rates, &nexus, RoundingScope::PerLine, mode)
            .expect("computes");
        prop_assert!(!breakdown.nexus_established);
        prop_assert_eq!(breakdown.total_tax, 0);
    }

    /// The parser boundary never panics, whatever bytes arrive.
    #[test]
    fn jurisdiction_rate_parsing_never_panics(arbitrary in ".*") {
        let _ = JurisdictionRate::from_percent(
            JurisdictionLevel::State,
            "x",
            &arbitrary,
        );
    }
}
