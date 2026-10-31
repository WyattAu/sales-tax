# sales-tax

US sales tax: the document arithmetic, with everything jurisdictional as an
input. Rates and taxability are inputs; the rate database is a data service,
not a library.

## Why US sales tax is not VAT with different field names

1. **Nexus gates collection**; *South Dakota v. Wayfair* (2018): economic
   nexus is a dollar or transaction threshold per state, set and revised by
   each state. The registry is an input; the breakdown reports whether
   collection applied at all.
2. **Prices are quoted pre-tax**; tax is added at the till. There is no
   VAT-style backwards division.
3. **Sales tax is never reclaimable**; a business eats the tax on its own
   purchases. There is no input-tax credit, unlike VAT. (This is why
   `ap-kit`'s `Recoverability` has no US counterpart.)
4. **Taxability varies per product, per state**; groceries, clothing,
   medicine. The decision is an input on every line; this crate will not
   infer it from a description.
5. **The rate is a stack**; state + county + city + special district, each
   remitted separately. Exact rational rates per level
   (`JurisdictionRate::from_percent`; `"6.25"` is 6.25%, i.e. a multiplier
   of 1/16, not 6.25×), with the breakdown naming each level's share.

## The rounding scope is explicit because the states differ

`PerDocument` (sum the taxable lines, round once per jurisdiction) vs
`PerLine` (round each line, then sum) genuinely differ; two $0.10 lines at
5% are £0.02 per-line and £0.01 per-document. The caller's filing rules
decide; the crate names the difference.

## Nexus without a rate stack is an error, not a zero

With nexus and no rates, returning zero would dress a missing data feed up
as a legitimate no-tax sale. It's refused. (Whether the supplied stack is
the right state's stack is the caller's data contract; rates carry no state
field to check against.)

## Licence

MIT OR Apache-2.0.
