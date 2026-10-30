//! Exemption certificates: the proof that a buyer doesn't owe tax.
//!
//! # Why the certificate is a typed input, not a flag
//!
//! A resale certificate is a *legal document* with an issuer, an expiry
//! date, and a statutory basis. Modeling it as a boolean `is_exempt: true`
//! would lose all of that: no way to audit which exemption was claimed,
//! no way to detect an expired certificate. The typed input carries both.
//!
//! # The boundary
//!
//! The crate verifies that the certificate is present and unexpired. It
//! does **not** verify that the certificate is genuine — that requires the
//! state's registry, which is the same data-service boundary as the rate
//! database.

/// An exemption certificate: the proof that a buyer doesn't owe tax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExemptionCertificate {
    /// The certificate's unique identifier.
    pub certificate_id: String,
    /// The statutory basis (e.g. "resale", "nonprofit", "direct-pay").
    pub exemption_code: String,
    /// The state that issued the certificate.
    pub state: String,
    /// The expiry date, `YYYY-MM-DD`.
    pub expiry: String,
}

impl ExemptionCertificate {
    /// Whether the certificate is valid on `date` (`YYYY-MM-DD`).
    ///
    /// A certificate is valid when its expiry is on or after the date.
    /// Lexicographic comparison is exact for `YYYY-MM-DD`.
    #[must_use]
    pub fn is_valid_on(&self, date: &str) -> bool {
        self.expiry.as_str() >= date
    }
}
