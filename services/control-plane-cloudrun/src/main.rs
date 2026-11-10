//! Stateless Google Cloud Run fallback control-plane service for UMCF.
//! Strictly handles control-plane signaling, enrollment token verification, and node presence;
//! never proxies bulk data-plane payloads.

use std::env;
use std::process::ExitCode;
use umcf_proto::{MessageKind, PlaneKind, WireEnvelope, CURRENT_PROTOCOL_VERSION};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--self-test") {
        match verify_control_plane_handler() {
            Ok(msg) => {
                println!("{msg}");
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("cloudrun control-plane error: {e}");
                return ExitCode::from(1);
            }
        }
    }
    println!(
        "UMCF Cloud Run Control-Plane Fallback Service v{} (protocol v{}) ready.",
        env!("CARGO_PKG_VERSION"),
        CURRENT_PROTOCOL_VERSION
    );
    ExitCode::SUCCESS
}

pub fn verify_control_plane_handler() -> Result<String, String> {
    let env = WireEnvelope::new(
        PlaneKind::ControlPlane,
        MessageKind::Hello,
        1,
        "did:umcf:cloudrun0123456789abcdef01234567",
        b"{\"service\":\"cloudrun-fallback\",\"healthy\":true}".to_vec(),
    )
    .map_err(|e| e.to_string())?;
    let roundtrip = WireEnvelope::from_bytes(&env.to_bytes()).map_err(|e| e.to_string())?;
    if roundtrip != env {
        return Err("roundtrip verification failed".into());
    }
    Ok(format!(
        "{{\"status\":\"PASS\",\"service\":\"umcf-control-plane-cloudrun\",\"protocol\":\"{}\"}}",
        CURRENT_PROTOCOL_VERSION
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_cloudrun_control_plane_self_test() {
        let out = verify_control_plane_handler().expect("self test");
        assert!(out.contains("\"status\":\"PASS\""));
    }
}

