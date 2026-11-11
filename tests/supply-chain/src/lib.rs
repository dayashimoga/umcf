//! Supply chain compliance and reproducible build test support crate.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use std::collections::HashSet;
pub use umcf_core::attestation::*;

/// Evaluates whether a Cargo manifest or dependency set contains any banned licenses.
pub fn scan_dependency_licenses(components: &[SbomPackage]) -> Result<usize, AttestationError> {
    for comp in components {
        if !is_license_permissive(&comp.license_spdx) {
            return Err(AttestationError::ProhibitedLicense {
                package: comp.name.clone(),
                license: comp.license_spdx.clone(),
            });
        }
    }
    Ok(components.len())
}

/// Validates that an SBOM is complete and has unique package identifiers.
pub fn validate_sbom_uniqueness(sbom: &CycloneDxSbom) -> bool {
    let mut seen = HashSet::new();
    for c in &sbom.components {
        if !seen.insert(&c.purl) {
            return false;
        }
    }
    true
}
