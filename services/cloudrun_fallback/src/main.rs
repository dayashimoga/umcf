//! Google Cloud Run stateless fallback control plane for UMCF.

use std::process::ExitCode;
use umcf_proto::{MessageKind, PlaneKind, WireEnvelope, CURRENT_PROTOCOL_VERSION};

fn main() -> ExitCode {
    println!(
        "UMCF Cloud Run Fallback Service v{} (protocol v{}) ready.",
        env!("CARGO_PKG_VERSION"),
        CURRENT_PROTOCOL_VERSION
    );
    ExitCode::SUCCESS
}

/// Validates fallback control-plane message reception.
pub fn verify_fallback_envelope() -> Result<bool, String> {
    let env = WireEnvelope::new(
        PlaneKind::ControlPlane,
        MessageKind::Hello,
        1,
        "did:umcf:cloudrunfallback0123456789abcdef",
        b"{\"fallback\":true}".to_vec(),
    )
    .map_err(|e| e.to_string())?;

    let bytes = env.to_bytes();
    let decoded = WireEnvelope::from_bytes(&bytes).map_err(|e| e.to_string())?;
    Ok(decoded == env)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_verify_fallback_envelope() {
        assert!(verify_fallback_envelope().expect("verification"));
    }
}
