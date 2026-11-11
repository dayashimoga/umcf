// top_tui.rs
// Terminal UI snapshot renderer for `umcf top` and `umcf cluster top`.

use umcf_sdk::{ClusterTelemetry, NodeDescriptor};

/// Renders a formatted terminal snapshot of live cluster state for a given terminal width and height.
#[must_use]
pub fn render_top_buffer(
    telemetry: &ClusterTelemetry,
    nodes: &[NodeDescriptor],
    width: usize,
    _height: usize,
) -> String {
    let w = width.max(80);
    let mut out = String::new();

    // 1. Header Banner
    let sep = "=".repeat(w);
    let title = " UMCF CLUSTER MONITOR (TOP) — v1.0.0 ";
    out.push_str(&sep);
    out.push('\n');
    out.push_str(&format!("{title:^w$}\n"));
    out.push_str(&sep);
    out.push('\n');

    // 2. Global Fabric Telemetry
    out.push_str(&format!(
        " Nodes: {} total ({} active) | Cores: {} | RAM: {:.1} GB | Storage: {:.1} / {:.1} GB | P95 RTT: {:.2} ms\n",
        telemetry.total_nodes,
        telemetry.active_workers,
        telemetry.aggregate_cpu_cores,
        telemetry.aggregate_ram_gb,
        telemetry.cluster_storage_used_gb,
        telemetry.cluster_storage_capacity_gb,
        telemetry.network_mesh_p95_rtt_ms,
    ));
    out.push_str(&"-".repeat(w));
    out.push('\n');

    // 3. Node Table Header
    out.push_str(&format!(
        "{:<24} {:<12} {:<18} {:<6} {:<16} {:<12} {:<8}\n",
        "NODE ID", "ROLE", "OS", "CORES", "GPU", "RAM (MB)", "STATUS"
    ));
    out.push_str(&"-".repeat(w));
    out.push('\n');

    // 4. Node Rows
    for node in nodes {
        let gpu = node.gpu_model.as_deref().unwrap_or("CPU Only");
        let ram_str = format!("{}/{}", node.ram_available_mb, node.ram_total_mb);
        out.push_str(&format!(
            "{:<24} {:<12} {:<18} {:<6} {:<16} {:<12} {:<8}\n",
            if node.node_id.len() > 24 {
                &node.node_id[..24]
            } else {
                &node.node_id
            },
            node.role,
            if node.os.len() > 18 {
                &node.os[..18]
            } else {
                &node.os
            },
            node.cpu_cores,
            if gpu.len() > 16 { &gpu[..16] } else { gpu },
            ram_str,
            node.status
        ));
    }

    out.push_str(&sep);
    out.push('\n');
    out.push_str("Press Ctrl+C or 'q' to exit live monitor.\n");

    out
}
