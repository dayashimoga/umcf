//! Automated v1.0 Release Readiness Auditor & Production Certification Command
//! Usage: `cargo run --bin umcf-certify-v1`
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![forbid(unsafe_code)]

use ed25519_dalek::SigningKey;
use rand_core::OsRng;
use release_cert::{GateStatus, MasterAuditReceipt, ReleaseAuditor};
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("================================================================================");
    println!("    UNIVERSAL MOBILE COMPUTE FABRIC (UMCF) — v1.0.0 RELEASE CERTIFICATION       ");
    println!("================================================================================");

    // Locate workspace root
    let root_path = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        PathBuf::from(manifest_dir)
            .parent()
            .and_then(|p| p.parent())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        PathBuf::from(".")
    };

    println!("Workspace Root: {}", root_path.display());
    println!("Auditing 10 Production Release Gates...\n");

    // Initialize Release Authority signing key
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    // Execute 10 Gates
    let receipt: MasterAuditReceipt = ReleaseAuditor::run_all_gates(&root_path, &signing_key);

    for gate in &receipt.gates {
        let status_str = match gate.status {
            GateStatus::Passed => "[PASSED]",
            GateStatus::Failed => "[FAILED]",
        };
        println!(
            "Gate {:02} {} — {}: {}",
            gate.gate_number, status_str, gate.name, gate.details
        );
    }

    println!("\n--------------------------------------------------------------------------------");
    println!(
        "Audit Result: {}/{} Gates Passed | Overall Status: {}",
        receipt.gates_passed, receipt.gates_total, receipt.overall_status
    );
    println!("Authority Public Key: {}", receipt.authority_public_key);
    println!("Authority Signature:  {}", receipt.authority_signature);
    println!("--------------------------------------------------------------------------------");

    // Write final signed certification receipt
    let p3_evidence_dir = root_path
        .join("artifacts")
        .join("evidence")
        .join("stage-08")
        .join("p3");
    let p3_stage_dir = root_path.join("artifacts").join("stage-08").join("p3");

    fs::create_dir_all(&p3_evidence_dir)?;
    fs::create_dir_all(&p3_stage_dir)?;

    let receipt_json = serde_json::to_string_pretty(&receipt)?;
    fs::write(
        p3_evidence_dir.join("FINAL_V1_0_CERTIFICATION_RECEIPT.json"),
        &receipt_json,
    )?;
    fs::write(
        p3_stage_dir.join("FINAL_V1_0_CERTIFICATION_RECEIPT.json"),
        &receipt_json,
    )?;

    if receipt.is_fully_passed() {
        println!("\n>>> UMCF v1.0.0 PRODUCTION CERTIFICATION PASSED (10/10 GATES VERIFIED) <<<\n");
        Ok(())
    } else {
        eprintln!("\n>>> UMCF v1.0.0 PRODUCTION CERTIFICATION FAILED <<<");
        std::process::exit(1);
    }
}
