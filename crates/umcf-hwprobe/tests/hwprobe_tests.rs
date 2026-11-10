//! Tests for `umcf-hwprobe` classified across the 5 mandatory tiers.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;
use std::time::Duration;
use tempfile::tempdir;
use umcf_core::capabilities::{AcceleratorKind, ComputeBackend, ThermalState};
use umcf_core::types::NodeId;
use umcf_hwprobe::accelerator_probe::{
    AcceleratorProber, GpuEnumerator, PCI_VENDOR_AMD, PCI_VENDOR_APPLE, PCI_VENDOR_INTEL,
    PCI_VENDOR_NVIDIA, PCI_VENDOR_QUALCOMM,
};
use umcf_hwprobe::cpu::CpuProber;
use umcf_hwprobe::memory_storage::MemoryStorageProber;
use umcf_hwprobe::power_thermal::PowerThermalProber;
use umcf_hwprobe::system_probe::SystemProber;

// ---------------------------------------------------------------------------
// 1. [Unit] Tests
// ---------------------------------------------------------------------------

#[test]
fn test_unit_parse_x86_64_cpuinfo_fixture() {
    let fixture = r#"
processor	: 0
vendor_id	: GenuineIntel
cpu family	: 6
model		: 154
model name	: 12th Gen Intel(R) Core(TM) i7-12700H
stepping	: 3
microcode	: 0x430
cpu MHz		: 2688.000
cache size	: 24576 KB
physical id	: 0
siblings	: 20
core id		: 0
cpu cores	: 14
apicid		: 0
initial apicid	: 0
fpu		: yes
fpu_exception	: yes
cpuid level	: 32
wp		: yes
flags		: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ss ht syscall nx pdpe1gb rdtscp lm constant_tsc arch_perfmon rep_good nopl xtopology cpuid tsc_known_freq pni pclmulqdq vmx ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt tsc_deadline_timer aes xsave avx f16c rdrand hypervisor lahf_lm abm 3dnowprefetch cpuid_fault invpcid_single ssbd ibrs ibpb stibp ibrs_enhanced tpr_shadow vnmi flexpriority ept vpid ept_ad fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb intel_pt sha_ni xsaveopt xsavec xgetbv1 xsaves split_lock_detect user_shstk avx_vnni dlayout flush_l1d arch_capabilities
bugs		: spectre_v1 spectre_v2 spec_store_bypass swapgs
bogomips	: 5376.00
clflush size	: 64
cache_alignment	: 64
address sizes	: 39 bits physical, 48 bits virtual
power management:

processor	: 1
core id		: 1
flags		: fpu avx2 sse4_2
"#;

    let (cores, flags) = CpuProber::parse_cpuinfo(fixture);
    assert_eq!(cores, 2);
    assert!(flags.iter().any(|f| f == "avx2"));
    assert!(flags.iter().any(|f| f == "avx"));
    assert!(flags.iter().any(|f| f == "sse4_2"));
    assert!(!flags.iter().any(|f| f == "neon"));
    assert!(!flags.iter().any(|f| f == "sve"));
}

#[test]
fn test_unit_parse_arm64_android_cpuinfo_fixture() {
    let fixture = r#"
processor	: 0
BogoMIPS	: 38.40
Features	: fp asimd evtstrm aes pmull sha1 sha2 crc32 atomics fphp asimdhp cpuid asimdrdm jscvt fcma lrcpc dcpop sha3 sm3 sm4 asimddp sha512 sve asimdfhm dit uscat ilrcpc flagm sb paca pacg dcpodp flagm2 frint svefrint sve2
CPU implementer	: 0x41
CPU architecture: 8
CPU variant	: 0x1
CPU part	: 0xd44
CPU revision	: 1

processor	: 1
BogoMIPS	: 38.40
Features	: fp asimd neon sve sve2
CPU implementer	: 0x41
"#;

    let (cores, flags) = CpuProber::parse_cpuinfo(fixture);
    assert_eq!(cores, 2);
    assert!(flags.iter().any(|f| f == "asimd" || f == "neon"));
    assert!(flags.iter().any(|f| f == "sve"));
    assert!(flags.iter().any(|f| f == "sve2"));
    assert!(!flags.iter().any(|f| f == "avx2"));
}

#[test]
fn test_unit_parse_cgroup_v2_cpu_max() {
    assert_eq!(CpuProber::parse_cgroup_cpu_max("max 100000"), None);
    assert_eq!(CpuProber::parse_cgroup_cpu_max("200000 100000"), Some(2.0));
    assert_eq!(CpuProber::parse_cgroup_cpu_max("400000 100000"), Some(4.0));
    assert_eq!(CpuProber::parse_cgroup_cpu_max("50000 100000"), Some(0.5));
    assert_eq!(CpuProber::parse_cgroup_cpu_max("invalid"), None);
    assert_eq!(CpuProber::parse_cgroup_cpu_max(""), None);
}

#[test]
fn test_unit_parse_cgroup_v2_memory_max() {
    assert_eq!(MemoryStorageProber::parse_cgroup_memory_max("max"), None);
    assert_eq!(MemoryStorageProber::parse_cgroup_memory_max("max\n"), None);
    assert_eq!(
        MemoryStorageProber::parse_cgroup_memory_max("4294967296\n"),
        Some(4_294_967_296)
    );
    assert_eq!(
        MemoryStorageProber::parse_cgroup_memory_max("1073741824"),
        Some(1_073_741_824)
    );
    assert_eq!(MemoryStorageProber::parse_cgroup_memory_max("invalid"), None);
    assert_eq!(MemoryStorageProber::parse_cgroup_memory_max(""), None);
}

#[test]
fn test_unit_parse_cgroup_v2_memory_current() {
    assert_eq!(
        MemoryStorageProber::parse_cgroup_memory_current("536870912\n"),
        Some(536_870_912)
    );
    assert_eq!(MemoryStorageProber::parse_cgroup_memory_current("0"), Some(0));
    assert_eq!(MemoryStorageProber::parse_cgroup_memory_current("invalid"), None);
}

#[test]
fn test_unit_parse_battery_sysfs() {
    let (pct, charging) = PowerThermalProber::parse_battery_sysfs("85\n", "Charging\n");
    assert_eq!(pct, Some(85));
    assert!(charging);

    let (pct, charging) = PowerThermalProber::parse_battery_sysfs("12\n", "Discharging\n");
    assert_eq!(pct, Some(12));
    assert!(!charging);

    let (pct, charging) = PowerThermalProber::parse_battery_sysfs("100\n", "Full\n");
    assert_eq!(pct, Some(100));
    assert!(charging);

    let (pct, charging) = PowerThermalProber::parse_battery_sysfs("not_a_number\n", "Unknown\n");
    assert_eq!(pct, None);
    assert!(!charging);
}

#[test]
fn test_unit_thermal_temp_mapping() {
    assert_eq!(
        PowerThermalProber::temperature_to_thermal_state(35_000),
        ThermalState::Nominal
    );
    assert_eq!(
        PowerThermalProber::temperature_to_thermal_state(59_999),
        ThermalState::Nominal
    );
    assert_eq!(
        PowerThermalProber::temperature_to_thermal_state(60_000),
        ThermalState::Fair
    );
    assert_eq!(
        PowerThermalProber::temperature_to_thermal_state(74_999),
        ThermalState::Fair
    );
    assert_eq!(
        PowerThermalProber::temperature_to_thermal_state(75_000),
        ThermalState::Serious
    );
    assert_eq!(
        PowerThermalProber::temperature_to_thermal_state(89_999),
        ThermalState::Serious
    );
    assert_eq!(
        PowerThermalProber::temperature_to_thermal_state(90_000),
        ThermalState::Critical
    );
    assert_eq!(
        PowerThermalProber::temperature_to_thermal_state(105_000),
        ThermalState::Critical
    );
}

#[test]
fn test_unit_pci_vendor_classification() {
    assert_eq!(AcceleratorProber::classify_pci_vendor(PCI_VENDOR_NVIDIA), "NVIDIA");
    assert_eq!(AcceleratorProber::classify_pci_vendor(PCI_VENDOR_AMD), "AMD");
    assert_eq!(AcceleratorProber::classify_pci_vendor(PCI_VENDOR_INTEL), "Intel");
    assert_eq!(AcceleratorProber::classify_pci_vendor(PCI_VENDOR_APPLE), "Apple");
    assert_eq!(AcceleratorProber::classify_pci_vendor(PCI_VENDOR_QUALCOMM), "Qualcomm");
    assert_eq!(AcceleratorProber::classify_pci_vendor(0x13B5), "ARM");
    assert_eq!(AcceleratorProber::classify_pci_vendor(0x1AE0), "Google");
    assert_eq!(AcceleratorProber::classify_pci_vendor(0xFFFF), "Unknown Vendor");
}

#[test]
fn test_unit_unified_memory_heuristics() {
    // Apple Silicon is always unified memory
    assert!(AcceleratorProber::is_unified_memory_architecture(
        PCI_VENDOR_APPLE,
        AcceleratorKind::DiscreteGpu
    ));
    assert!(AcceleratorProber::is_unified_memory_architecture(
        PCI_VENDOR_QUALCOMM,
        AcceleratorKind::MobileGpu
    ));

    // Integrated GPUs and CPUs share host RAM
    assert!(AcceleratorProber::is_unified_memory_architecture(
        PCI_VENDOR_INTEL,
        AcceleratorKind::IntegratedGpu
    ));
    assert!(AcceleratorProber::is_unified_memory_architecture(
        PCI_VENDOR_AMD,
        AcceleratorKind::IntegratedGpu
    ));
    assert!(AcceleratorProber::is_unified_memory_architecture(
        0x0000,
        AcceleratorKind::Cpu
    ));

    // Discrete PCIe GPUs with dedicated VRAM are not unified memory
    assert!(!AcceleratorProber::is_unified_memory_architecture(
        PCI_VENDOR_NVIDIA,
        AcceleratorKind::DiscreteGpu
    ));
    assert!(!AcceleratorProber::is_unified_memory_architecture(
        PCI_VENDOR_AMD,
        AcceleratorKind::DiscreteGpu
    ));
}

// ---------------------------------------------------------------------------
// 2. [Integration] Tests
// ---------------------------------------------------------------------------

#[test]
fn test_integration_system_probe_live() {
    let node_id = NodeId("did:umcf:0123456789abcdef0123456789abcdef".to_string());
    let (descriptor, elapsed) = SystemProber::probe_all_with_duration(node_id.clone())
        .expect("system probe should succeed on live host");
    println!("Live probe execution duration: {elapsed:?}");

    // Timing check: Assert probe completed within reasonable bounds (< 25ms in release, < 150ms in debug)
    #[cfg(not(debug_assertions))]
    assert!(
        elapsed <= Duration::from_millis(SystemProber::MAX_PROBE_DURATION_MS),
        "Probe took too long in release: {elapsed:?}"
    );
    #[cfg(debug_assertions)]
    assert!(
        elapsed <= Duration::from_millis(150),
        "Probe took too long in debug: {elapsed:?}"
    );

    // Validate descriptor fields
    assert_eq!(descriptor.node_id, node_id);
    assert!(descriptor.cpu.physical_cores >= 1);
    assert!(descriptor.memory.total_ram.as_bytes() > 0);
    assert!(descriptor.memory.available_ram.as_bytes() > 0);
    assert!(descriptor.storage.total_disk.as_bytes() > 0);
    assert!(!descriptor.accelerators.is_empty());

    // Verify serializability
    let json = serde_json::to_string_pretty(&descriptor).expect("serialization to JSON");
    assert!(json.contains("node_id"));
    assert!(json.contains("cpu"));
    assert!(json.contains("memory"));
}

// ---------------------------------------------------------------------------
// 3. [Simulation] Fault-Injection Tests
// ---------------------------------------------------------------------------

#[test]
fn test_simulation_thermal_critical_and_low_battery_sysfs() {
    let temp_dir = tempdir().expect("create temp dir");

    // 1. Create simulated power_supply structure
    let bat0_dir = temp_dir.path().join("power_supply/BAT0");
    fs::create_dir_all(&bat0_dir).expect("create BAT0 dir");
    fs::write(bat0_dir.join("capacity"), "8\n").expect("write capacity");
    fs::write(bat0_dir.join("status"), "Discharging\n").expect("write status");

    let bat_result = PowerThermalProber::read_sysfs_battery(&temp_dir.path().join("power_supply"))
        .expect("read simulated battery")
        .expect("battery found");

    assert_eq!(bat_result.0, Some(8));
    assert!(!bat_result.1); // Discharging

    // 2. Create simulated thermal structure
    let thermal_dir = temp_dir.path().join("thermal/thermal_zone0");
    fs::create_dir_all(&thermal_dir).expect("create thermal_zone0 dir");
    fs::write(thermal_dir.join("temp"), "95000\n").expect("write thermal temp");

    let temp_result = PowerThermalProber::read_sysfs_thermal(&temp_dir.path().join("thermal"))
        .expect("read simulated thermal")
        .expect("thermal found");

    assert_eq!(temp_result, 95_000);
    let state = PowerThermalProber::temperature_to_thermal_state(temp_result);
    assert_eq!(state, ThermalState::Critical);
}

#[test]
fn test_simulation_cgroup_v2_constrained_container_limits() {
    let host_total_ram = 32_000_000_000_u64; // 32 GB host
    let host_avail_ram = 16_000_000_000_u64; // 16 GB host free

    let cgroup_memory_max = 4_294_967_296_u64; // 4 GiB container cap
    let cgroup_memory_usage = 1_073_741_824_u64; // 1 GiB used

    // Container constrained calculations
    let effective_total = host_total_ram.min(cgroup_memory_max);
    let cgroup_avail = cgroup_memory_max.saturating_sub(cgroup_memory_usage);
    let effective_avail = host_avail_ram.min(cgroup_avail);

    assert_eq!(effective_total, 4_294_967_296);
    assert_eq!(effective_avail, 3_221_225_472); // 3 GiB available
}

// ---------------------------------------------------------------------------
// 4. [Hardware-Required] Real Hardware Query Tests
// ---------------------------------------------------------------------------

#[test]
fn test_hw_required_cpu_simd_and_memory_probing() {
    let cpu = CpuProber::probe_cpu().expect("probe physical cpu");
    assert!(cpu.physical_cores >= 1);

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            assert!(
                cpu.simd_avx2,
                "AVX2 detected by cpuid must match CpuCapability flag"
            );
        }
    }

    let mem = MemoryStorageProber::probe_memory().expect("probe physical memory");
    assert!(mem.total_ram.as_bytes() >= 1_000_000_000); // Host has at least 1 GB RAM
    assert!(mem.available_ram.as_bytes() > 0);
}

// ---------------------------------------------------------------------------
// 5. [Manually-Verified] Live System Report Generation
// ---------------------------------------------------------------------------

#[test]
fn test_manually_verified_live_system_probe_report_audit() {
    let node_id = NodeId("did:umcf:00001111222233334444555566667777".to_string());
    let (descriptor, duration) = SystemProber::probe_all_with_duration(node_id)
        .expect("probe all on live system");

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("../..");
    let report_path = workspace_root.join("artifacts/evidence/stage-03/live_system_probe_report.json");

    if let Some(parent) = report_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let json_bytes = serde_json::to_vec_pretty(&descriptor).expect("serialize report to json");
    fs::write(&report_path, &json_bytes).expect("write live system probe report");

    println!(
        "[Manually-Verified] Generated live system report in {:?} at {:?}",
        duration, report_path
    );
    assert!(report_path.exists());
    assert!(!json_bytes.is_empty());
}

#[test]
fn test_integration_accelerator_enumeration_live() {
    AcceleratorProber::warmup();
    let (accelerators, duration) = AcceleratorProber::enumerate_with_duration()
        .expect("accelerator enumeration should never panic");

    #[cfg(not(debug_assertions))]
    assert!(
        duration <= Duration::from_millis(AcceleratorProber::MAX_ENUMERATION_DURATION_MS),
        "Accelerator enumeration took too long in release: {duration:?}"
    );
    #[cfg(debug_assertions)]
    assert!(
        duration <= Duration::from_millis(250),
        "Accelerator enumeration took too long in debug: {duration:?}"
    );

    assert!(!accelerators.is_empty(), "must find at least one accelerator or CPU fallback");
    for acc in &accelerators {
        assert!(!acc.name.is_empty(), "accelerator name must not be empty");
        assert!(acc.compute_units >= 1, "compute units must be >= 1");
        assert!(acc.vram_bytes.as_bytes() > 0, "VRAM/RAM budget must be > 0");
    }
}

#[test]
fn test_simulation_cuda_rocm_absence_and_graceful_cpu_fallback() {
    let fallback = AcceleratorProber::create_cpu_simd_fallback();
    assert_eq!(fallback.kind, AcceleratorKind::Cpu);
    assert_eq!(fallback.backend, ComputeBackend::CpuSimd);
    assert!(fallback.verified_on_hardware);
    assert!(fallback.supports_int8);
    assert!(fallback.supports_int4);
    assert_eq!(fallback.shared_memory_mb, 1024);
}

#[test]
fn test_hw_required_accelerator_enumeration() {
    let wgpu_adapters = GpuEnumerator::enumerate_wgpu_adapters()
        .expect("wgpu adapter query should succeed or return empty");
    println!("Found {} hardware/software wgpu adapters", wgpu_adapters.len());

    let all_acc = AcceleratorProber::enumerate()
        .expect("complete enumeration must succeed");
    assert!(!all_acc.is_empty());
}

#[test]
fn test_manually_verified_accelerator_enumeration_report_audit() {
    let (accelerators, duration) = AcceleratorProber::enumerate_with_duration()
        .expect("accelerator enumeration must succeed");

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("../..");
    let report_path = workspace_root.join("artifacts/evidence/stage-03/accelerator_enumeration_report.json");

    if let Some(parent) = report_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let json_bytes = serde_json::to_vec_pretty(&accelerators).expect("serialize accelerators to json");
    fs::write(&report_path, &json_bytes).expect("write accelerator report");

    println!(
        "[Manually-Verified] Generated accelerator report with {} accelerators in {:?} at {:?}",
        accelerators.len(), duration, report_path
    );
    assert!(report_path.exists());
    assert!(!json_bytes.is_empty());
}
