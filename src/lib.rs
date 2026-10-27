//! US sales tax: the document arithmetic, with everything jurisdictional as
//! an input.
//!
//! # Why US sales tax is not VAT with different field names
//!
//! Five structural differences, each of which shapes the types here:
//!
//! 1. **Nexus gates collection.** A seller collects only where they have
//!    nexus — physical presence, or economic nexus after *South Dakota v.
//!    Wayfair* (2018): typically a dollar or transaction threshold per state,
//!    set and revised by each state. There is no VAT-style universal
//!    obligation, so the registry is an input and the breakdown reports
//!    whether collection applied at all.
//! 2. **Prices are quoted pre-tax.** Tax is *added* at the till; there is no
//!    VAT-style backwards division. The customer's total is net plus tax.
//! 3. **Sales tax is never reclaimable.** A business pays tax on its own
//!    purchases and eats it — there is no input-tax credit, unlike VAT where
//!    recoverability is the whole game. (This is why `ap-kit`'s
//!    `Recoverability` has no US counterpart.)
//! 4. **Taxability varies per product, per state.** Groceries, clothing,
//!    medicine — each state decides differently, so the taxable/not decision
//!    is an input on every line, the way recoverability is an input on every
//!    AP bill.
//! 5. **The rate is a stack.** State + county + city + special district,
//!    each remitted separately. The rates change monthly across thousands of
//!    jurisdictions — which is a *data-service* business (Avalara, TaxJar),
//!    not a library. This crate takes exact rational rates per jurisdiction
//!    and reports the breakdown per jurisdiction; where the rates come from
//!    is explicitly someone else's problem.
//!
//! # What the arithmetic guarantees
//!
//! Rates are exact rationals (`fx_kit::Rate`), every jurisdiction's tax is
//! rounded **once** through the integer path, and the rounding *scope* is
//! explicit — per document or per line — because states differ and the
//! difference is real pennies (see [`RoundingScope`]). The breakdown names
//! each jurisdiction's share, because the filing remits to each.

use fx_kit::Rate;
use vat_rules::TieMode;

/// Why a tax computation cannot proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SalesTaxError {
    /// Nexus is established for the destination state, but no rates were
    /// supplied. A missing rate stack is an input error, not a zero.
    MissingRate {
        /// The destination state that had no rates.
        state: String,
    },
    /// A line's net amount was negative.
    ///
    /// Credit memos are their own documents in the US model; a negative
    /// line in a sale is a data error, not a credit.
    NegativeLine {
        /// The offending line's index.
        index: usize,
    },
}

/// Which level of government a rate belongs to.
///
/// The filing remits to each separately — a Texas sale's 8.25% is the state's
/// 6.25% plus a local's 2.00%, and they are not interchangeable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum JurisdictionLevel {
    /// The state's own rate.
    State,
    /// A county rate.
    County,
    /// A city rate.
    City,
    /// A special-purpose district (transit, stadium, …).
    SpecialDistrict,
}

impl JurisdictionLevel {
    /// Every level, in filing order.
    pub const ALL: [Self; 4] = [Self::State, Self::County, Self::City, Self::SpecialDistrict];
}

/// One jurisdiction's exact rate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JurisdictionRate {
    /// Which level this rate is for.
    pub level: JurisdictionLevel,
    /// The jurisdiction's name, for the breakdown.
    pub name: String,
    /// The rate, exact: `"8.25"`% → 33/400.
    pub rate: Rate,
}

impl JurisdictionRate {
    /// A jurisdiction rate from its **quoted percent** — `"6.25"` meaning
    /// 6.25%, i.e. a multiplier of 1/16. US rate data is quoted as percents
    /// by universal convention; parsing it as a multiplier would make every
    /// rate a hundred times too large, which is the first bug a
    /// percent-vs-multiplier confusion produces.
    pub fn from_percent(
        level: JurisdictionLevel,
        name: &str,
        quoted: &str,
    ) -> Result<Self, fx_kit::FxError> {
        Ok(Self {
            level,
            name: name.to_string(),
            rate: Rate::parse_percent(quoted)?,
        })
    }
}

/// The states where the seller has nexus and therefore collects.
///
/// Populated from the seller's own registrations — economic nexus thresholds
/// are set per state, revised per year, and enforced per seller; the crate
/// does not encode them, because a hardcoded threshold table would be wrong
/// the first month it shipped.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NexusRegistry {
    states: std::collections::HashSet<String>,
}

impl NexusRegistry {
    /// A registry from two-letter state codes.
    pub fn new(states: &[String]) -> Self {
        Self {
            states: states.iter().map(|s| s.to_uppercase()).collect(),
        }
    }

    /// Whether the seller collects in `state`.
    #[must_use]
    pub fn has_nexus(&self, state: &str) -> bool {
        self.states.contains(&state.to_uppercase())
    }
}

/// One sale line, with its taxability decision.
///
/// The decision is an input: whether groceries, clothing under a threshold,
/// or medicine are taxable is state law, and the caller's product catalog
/// owns it. This crate will not infer taxability from a description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaleLine {
    /// The line's net amount, minor units, never negative. Pre-tax: US
    /// prices are quoted before tax.
    pub net_minor: i64,
    /// Whether this line is taxable in the destination state.
    pub taxable: bool,
}

/// Where the rounding happens. The states differ; the caller's filing rules
/// decide, and the difference is real pennies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingScope {
    /// Sum the taxable lines, apply the rate, round once per jurisdiction —
    /// the common default for invoices.
    PerDocument,
    /// Tax each taxable line and round it individually, then sum — the
    /// per-line method some states and platforms require.
    PerLine,
}

/// The computed tax, per jurisdiction and in total.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SalesTaxBreakdown {
    /// Whether collection applied at all — `false` means the destination
    /// state is outside the seller's nexus, and every figure is zero for
    /// that *legal* reason, not an optimization.
    pub nexus_established: bool,
    /// The destination state.
    pub destination_state: String,
    /// Tax per jurisdiction, in the order the rates were supplied, minor
    /// units.
    pub per_jurisdiction: Vec<(JurisdictionLevel, String, i64)>,
    /// Per-line tax, in input order — the audit trail for a per-line-scope
    /// filing. All zeros under [`RoundingScope::PerDocument`], which rounds
    /// at the document level.
    pub per_line: Vec<i64>,
    /// Total tax across all jurisdictions, minor units.
    pub total_tax: i64,
}

/// Compute the sales tax for a transaction shipped to `destination_state`.
///
/// Collection is gated on nexus: outside the registry, the breakdown reports
/// `nexus_established: false` with zero figures — which is the *legal*
/// answer, and is reported as such rather than folded into a silent zero.
/// With nexus, every supplied rate taxes the transaction at its own rate
/// with its own single rounding.
pub fn compute_tax(
    lines: &[SaleLine],
    destination_state: &str,
    rates: &[JurisdictionRate],
    nexus: &NexusRegistry,
    scope: RoundingScope,
    mode: TieMode,
) -> Result<SalesTaxBreakdown, SalesTaxError> {
    let nexus_established = nexus.has_nexus(destination_state);
    let mut per_jurisdiction: Vec<(JurisdictionLevel, String, i64)> = Vec::new();
    let mut per_line = vec![0i64; lines.len()];
    let mut total_tax = 0i64;

    if !nexus_established {
        return Ok(SalesTaxBreakdown {
            nexus_established,
            destination_state: destination_state.to_string(),
            per_jurisdiction,
            per_line,
            total_tax,
        });
    }

    if rates.is_empty() {
        return Err(SalesTaxError::MissingRate {
            state: destination_state.to_string(),
        });
    }

    for (index, line) in lines.iter().enumerate() {
        if line.net_minor < 0 {
            return Err(SalesTaxError::NegativeLine { index });
        }
    }

    for rate_entry in rates {
        let mut jurisdiction_tax = 0i64;
        match scope {
            RoundingScope::PerDocument => {
                // Sum the taxable nets, apply the rate, round once. The
                // pennies that per-line rounding would spill stay in the
                // transaction until this single rounding.
                let taxable_net: i64 = lines
                    .iter()
                    .filter(|l| l.taxable)
                    .map(|l| l.net_minor)
                    .sum();
                jurisdiction_tax = rate_entry.rate.restate(taxable_net, mode);
            }
            RoundingScope::PerLine => {
                for (index, line) in lines.iter().enumerate() {
                    if !line.taxable {
                        continue;
                    }
                    let line_tax = rate_entry.rate.restate(line.net_minor, mode);
                    jurisdiction_tax += line_tax;
                    if let Some(slot) = per_line.get_mut(index) {
                        *slot += line_tax;
                    }
                }
            }
        }
        total_tax += jurisdiction_tax;
        per_jurisdiction.push((rate_entry.level, rate_entry.name.clone(), jurisdiction_tax));
    }

    Ok(SalesTaxBreakdown {
        nexus_established,
        destination_state: destination_state.to_string(),
        per_jurisdiction,
        per_line,
        total_tax,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn rate(level: JurisdictionLevel, name: &str, quoted: &str) -> JurisdictionRate {
        JurisdictionRate::from_percent(level, name, quoted).expect("a valid quoted rate")
    }

    /// Texas's textbook stack: 6.25% state + 2.00% local.
    fn texas_rates() -> Vec<JurisdictionRate> {
        vec![
            rate(JurisdictionLevel::State, "Texas", "6.25"),
            rate(
                JurisdictionLevel::SpecialDistrict,
                "Transit authority",
                "2.00",
            ),
        ]
    }

    fn sale_lines() -> Vec<SaleLine> {
        // $10,000.00 net.
        vec![SaleLine {
            net_minor: 1_000_000,
            taxable: true,
        }]
    }

    #[test]
    fn the_texas_stack_taxes_the_document_at_eight_twenty_five() {
        // $100,000.00 minor = $1,000.00 at 6.25% = $62.50; at 2.00% = $20.00.
        let nexus = NexusRegistry::new(&["TX".to_string()]);
        let breakdown = compute_tax(
            &sale_lines(),
            "TX",
            &texas_rates(),
            &nexus,
            RoundingScope::PerDocument,
            TieMode::HalfUp,
        )
        .expect("computes");
        let state_tax = breakdown
            .per_jurisdiction
            .first()
            .expect("two jurisdictions")
            .2;
        let local_tax = breakdown
            .per_jurisdiction
            .get(1)
            .expect("two jurisdictions")
            .2;
        assert_eq!(state_tax, 62_500);
        assert_eq!(local_tax, 20_000);
        assert_eq!(breakdown.total_tax, 82_500);
        assert!(breakdown.nexus_established);
    }

    #[test]
    fn no_nexus_means_zero_for_a_legal_reason_and_it_is_reported() {
        // Selling into California with no CA nexus: no collection obligation.
        let nexus = NexusRegistry::new(&["TX".to_string()]);
        let breakdown = compute_tax(
            &sale_lines(),
            "CA",
            &texas_rates(),
            &nexus,
            RoundingScope::PerDocument,
            TieMode::HalfUp,
        )
        .expect("computes");
        assert!(!breakdown.nexus_established);
        assert_eq!(breakdown.total_tax, 0);
        assert!(breakdown.per_jurisdiction.is_empty());
    }

    #[test]
    fn nexus_without_any_rate_stack_is_an_input_error_not_a_zero() {
        // The one rate check this crate can make: with nexus and an empty
        // stack there is nothing to tax with, and returning zero would dress
        // a missing data feed up as a legitimate no-tax sale. (Whether the
        // supplied stack is the *right state's* stack is the caller's data
        // contract — the rates carry no state field to check against.)
        let nexus = NexusRegistry::new(&["NY".into()]);
        assert_eq!(
            compute_tax(
                &sale_lines(),
                "NY",
                &[],
                &nexus,
                RoundingScope::PerDocument,
                TieMode::HalfUp
            ),
            Err(SalesTaxError::MissingRate { state: "NY".into() })
        );
    }

    #[test]
    fn an_untaxable_line_contributes_nothing_under_both_scopes() {
        // Groceries in most states: on the receipt, out of the tax.
        let lines = vec![
            SaleLine {
                net_minor: 1_000_000, // $10,000.00 taxable
                taxable: true,
            },
            SaleLine {
                net_minor: 400_000, // $4,000.00 groceries
                taxable: false,
            },
        ];
        let nexus = NexusRegistry::new(&["TX".to_string()]);
        for scope in [RoundingScope::PerDocument, RoundingScope::PerLine] {
            let breakdown =
                compute_tax(&lines, "TX", &texas_rates(), &nexus, scope, TieMode::HalfUp)
                    .expect("computes");
            assert_eq!(
                breakdown.total_tax, 82_500,
                "the untaxable line is out, whichever scope"
            );
        }
    }

    #[test]
    fn the_scopes_genuinely_differ_and_the_difference_is_named() {
        // Two lines of $0.10 at 5%: each line's tax is exactly half a cent.
        // Per-line rounds each up (£0.01 + £0.01); per-document rounds the
        // £0.01 total once. One penny apart — the difference the states'
        // rules argue about, and the reason the scope is explicit.
        let lines = vec![
            SaleLine {
                net_minor: 10,
                taxable: true,
            },
            SaleLine {
                net_minor: 10,
                taxable: true,
            },
        ];
        let rates = vec![rate(JurisdictionLevel::State, "Test", "5")];
        let nexus = NexusRegistry::new(&["TX".to_string()]);

        let per_line = compute_tax(
            &lines,
            "TX",
            &rates,
            &nexus,
            RoundingScope::PerLine,
            TieMode::HalfUp,
        )
        .expect("computes");
        assert_eq!(per_line.total_tax, 2);

        let per_document = compute_tax(
            &lines,
            "TX",
            &rates,
            &nexus,
            RoundingScope::PerDocument,
            TieMode::HalfUp,
        )
        .expect("computes");
        assert_eq!(per_document.total_tax, 1);

        // And the per-line breakdown is the audit trail: both lines' tax is
        // visible individually.
        assert_eq!(per_line.per_line, vec![1, 1]);
    }

    #[test]
    fn a_negative_line_is_a_data_error_not_a_credit() {
        let lines = vec![SaleLine {
            net_minor: -100,
            taxable: true,
        }];
        let nexus = NexusRegistry::new(&["TX".to_string()]);
        assert_eq!(
            compute_tax(
                &lines,
                "TX",
                &texas_rates(),
                &nexus,
                RoundingScope::PerDocument,
                TieMode::HalfUp
            ),
            Err(SalesTaxError::NegativeLine { index: 0 })
        );
    }
}
