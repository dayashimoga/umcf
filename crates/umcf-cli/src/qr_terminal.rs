// qr_terminal.rs
// UTF-8 Half-Block Terminal QR Code Renderer for `umcf cluster pair --qr`.

use qrcode::{Color, EcLevel, QrCode};

/// Renders a text string into an ultra-compact UTF-8 half-block terminal QR code.
///
/// Uses half-block Unicode characters (`▀`, `▄`, `█`, ` `) to compress two vertical
/// QR modules into a single character cell, preserving a 1:1 square aspect ratio in terminals.
pub fn render_qr_halfblock(text: &str) -> Result<String, String> {
    let code = QrCode::with_error_correction_level(text.as_bytes(), EcLevel::M)
        .map_err(|e| format!("Failed to generate QR code: {e}"))?;

    let width = code.width();
    let modules = code.to_colors();

    let mut output = String::new();
    let border = 2; // Quiet zone

    let full_width = width + border * 2;
    let full_height = width + border * 2;

    // Helper to sample module color with quiet zone
    let is_dark = |x: usize, y: usize| -> bool {
        if x < border || x >= border + width || y < border || y >= border + width {
            return false; // Quiet zone is light (false)
        }
        let mx = x - border;
        let my = y - border;
        modules[my * width + mx] == Color::Dark
    };

    // Iterate 2 rows at a time
    let mut y = 0;
    while y < full_height {
        for x in 0..full_width {
            let top = is_dark(x, y);
            let bottom = if y + 1 < full_height {
                is_dark(x, y + 1)
            } else {
                false
            };

            // Using standard terminal inverted half-blocks for black-on-white QR scanner compatibility:
            // Dark modules are rendered as spaces or lower/upper blocks against bright foreground
            match (top, bottom) {
                (true, true) => output.push(' '),
                (true, false) => output.push('▄'),
                (false, true) => output.push('▀'),
                (false, false) => output.push('█'),
            }
        }
        output.push('\n');
        y += 2;
    }

    Ok(output)
}

