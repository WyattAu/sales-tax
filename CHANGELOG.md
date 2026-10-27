# Changelog

All notable changes to this project are documented here. Format: [Keep a
Changelog](https://keepachangelog.com/) — versions follow [semver](https://semver.org).

## [Unreleased]

## [0.1.0] - 2026-10-27

Initial release.

### Added

- `compute_tax` — nexus-gated, destination-sourced sales tax over a line
  list: per-jurisdiction breakdown, per-line audit trail, explicit rounding
  scope (`PerDocument` vs `PerLine` — two $0.10 lines at 5% genuinely
  differ).
- `JurisdictionRate::from_percent` — quoted percentages ("6.25") to exact
  reduced rationals via `fx_kit::Rate`, because parsing a percent as a
  multiplier makes every rate a hundred times too large.
- `NexusRegistry` — the seller's registrations; collection gated on nexus,
  with `nexus_established` reported so a legal zero is distinguishable from
  a missing feed.
- Nexus with an empty rate stack, negative lines: errors, not silent zeros.
