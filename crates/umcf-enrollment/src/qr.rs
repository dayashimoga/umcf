//! SVG and Unicode terminal QR code matrix rendering for UMCF enrollment tokens.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use crate::error::EnrollmentError;
use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};
use std::path::Path;

/// QR Code matrix generator and visual renderer.
#[derive(Debug, Clone, Copy, Default)]
pub struct QrRenderer;

impl QrRenderer {
    /// Renders an enrollment URI as an SVG string at Error Correction Level M.
    pub fn to_svg(uri: &str) -> Result<String, EnrollmentError> {
        let code = QrCode::with_error_correction_level(uri.as_bytes(), EcLevel::M)
            .map_err(|e| EnrollmentError::QrGenerationFailed(e.to_string()))?;

        let svg_image = code
            .render::<svg::Color>()
            .min_dimensions(320, 320)
            .quiet_zone(true)
            .dark_color(svg::Color("#0f172a"))
            .light_color(svg::Color("#ffffff"))
            .build();

        Ok(svg_image)
    }

    /// Renders an enrollment URI as a compact Unicode terminal block string using double-height blocks.
    pub fn to_unicode_blocks(uri: &str) -> Result<String, EnrollmentError> {
        let code = QrCode::with_error_correction_level(uri.as_bytes(), EcLevel::M)
            .map_err(|e| EnrollmentError::QrGenerationFailed(e.to_string()))?;

        let unicode_str = code
            .render::<char>()
            .quiet_zone(true)
            .module_dimensions(2, 1)
            .build();

        Ok(unicode_str)
    }

    /// Writes an SVG QR code directly to disk at the specified filesystem path.
    pub fn to_svg_file(uri: &str, path: &Path) -> Result<(), EnrollmentError> {
        let svg = Self::to_svg(uri)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| EnrollmentError::SerializationError(e.to_string()))?;
        }
        std::fs::write(path, svg).map_err(|e| EnrollmentError::SerializationError(e.to_string()))?;
        Ok(())
    }
}
