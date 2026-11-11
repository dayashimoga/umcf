// erasure_shard_matrix_screen.dart
// UMCF Stage 07 Phase 1 Subphase 2 (S7.P1.SP2)
// Systematic Reed-Solomon GF(2^8) Erasure Coding & Encrypted Shard Placement Matrix Screen.
// Adheres strictly to Flutter multiplatform standards (Android, iOS, Windows, Linux, macOS),
// Riverpod state management, responsive breakpoints, and zero decorative controls.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

enum RedundancyMode { rs42, rs63, rep3 }

enum ShardDurabilityStatus { healthy, offline, healing }

class ShardModel {
  final int index;
  final String tag;
  final bool isParity;
  final String assignedNode;
  final String nodeId;
  final String failureDomain;
  final bool isAcPowered;
  final int batteryPct;
  final String shardHash;
  final int sizeKb;
  final ShardDurabilityStatus status;

  const ShardModel({
    required this.index,
    required this.tag,
    required this.isParity,
    required this.assignedNode,
    required this.nodeId,
    required this.failureDomain,
    required this.isAcPowered,
    required this.batteryPct,
    required this.shardHash,
    required this.sizeKb,
    required this.status,
  });

  ShardModel copyWith({
    String? assignedNode,
    String? nodeId,
    String? failureDomain,
    bool? isAcPowered,
    int? batteryPct,
    ShardDurabilityStatus? status,
  }) {
    return ShardModel(
      index: index,
      tag: tag,
      isParity: isParity,
      assignedNode: assignedNode ?? this.assignedNode,
      nodeId: nodeId ?? this.nodeId,
      failureDomain: failureDomain ?? this.failureDomain,
      isAcPowered: isAcPowered ?? this.isAcPowered,
      batteryPct: batteryPct ?? this.batteryPct,
      shardHash: shardHash,
      sizeKb: sizeKb,
      status: status ?? this.status,
    );
  }
}

class ErasureMatrixState {
  final RedundancyMode mode;
  final List<ShardModel> shards;
  final List<String> repairLogs;
  final bool isHealing;

  const ErasureMatrixState({
    required this.mode,
    required this.shards,
    required this.repairLogs,
    required this.isHealing,
  });

  int get totalShards => shards.length;
  int get healthyShards => shards.where((s) => s.status == ShardDurabilityStatus.healthy).length;
  int get offlineShards => shards.where((s) => s.status == ShardDurabilityStatus.offline).length;
  int get requiredK => mode == RedundancyMode.rs42 ? 4 : mode == RedundancyMode.rs63 ? 6 : 1;
  int get toleratesOutages => (healthyShards - requiredK).clamp(0, 99);

  bool get isDegraded => healthyShards < totalShards && healthyShards >= requiredK;
  bool get isUnrecoverable => healthyShards < requiredK;

  ErasureMatrixState copyWith({
    RedundancyMode? mode,
    List<ShardModel>? shards,
    List<String>? repairLogs,
    bool? isHealing,
  }) {
    return ErasureMatrixState(
      mode: mode ?? this.mode,
      shards: shards ?? this.shards,
      repairLogs: repairLogs ?? this.repairLogs,
      isHealing: isHealing ?? this.isHealing,
    );
  }
}

class ErasureMatrixNotifier extends StateNotifier<ErasureMatrixState> {
  ErasureMatrixNotifier() : super(_initialState());

  static const List<Map<String, dynamic>> _nodes = [
    {"name": "desktop-alpha", "id": "did:umcf:desktop_01", "domain": "subnet:192.168.1.0/24", "ac": true, "bat": 100},
    {"name": "server-edge-01", "id": "did:umcf:server_02", "domain": "subnet:192.168.2.0/24", "ac": true, "bat": 100},
    {"name": "macbook-m3-pro", "id": "did:umcf:macbook_03", "domain": "subnet:192.168.1.0/24", "ac": true, "bat": 95},
    {"name": "pixel-8-pro", "id": "did:umcf:pixel_04", "domain": "subnet:192.168.3.0/24", "ac": false, "bat": 82},
    {"name": "workstation-xeon", "id": "did:umcf:workstation_05", "domain": "rack:alpha_dc1", "ac": true, "bat": 100},
    {"name": "galaxy-tab-s9", "id": "did:umcf:tablet_06", "domain": "subnet:192.168.3.0/24", "ac": false, "bat": 65},
    {"name": "thinkpad-x1-spare", "id": "did:umcf:thinkpad_07", "domain": "subnet:192.168.4.0/24", "ac": true, "bat": 100},
    {"name": "nuc-homelab-spare", "id": "did:umcf:nuc_08", "domain": "subnet:192.168.2.0/24", "ac": true, "bat": 100},
    {"name": "surface-laptop-spare", "id": "did:umcf:surface_09", "domain": "subnet:192.168.4.0/24", "ac": false, "bat": 78},
  ];

  static ErasureMatrixState _initialState() {
    return ErasureMatrixState(
      mode: RedundancyMode.rs42,
      shards: _buildShards(RedundancyMode.rs42),
      repairLogs: const [],
      isHealing: false,
    );
  }

  static List<ShardModel> _buildShards(RedundancyMode mode) {
    final list = <ShardModel>[];
    if (mode == RedundancyMode.rs42) {
      for (int i = 0; i < 4; i++) {
        final n = _nodes[i];
        list.add(ShardModel(
          index: i,
          tag: "D$i",
          isParity: false,
          assignedNode: n["name"] as String,
          nodeId: n["id"] as String,
          failureDomain: n["domain"] as String,
          isAcPowered: n["ac"] as bool,
          batteryPct: n["bat"] as int,
          shardHash: "blake3:8f${(i + 1) * 11}a2b3",
          sizeKb: 256,
          status: ShardDurabilityStatus.healthy,
        ));
      }
      for (int i = 0; i < 2; i++) {
        final n = _nodes[4 + i];
        list.add(ShardModel(
          index: 4 + i,
          tag: "P$i",
          isParity: true,
          assignedNode: n["name"] as String,
          nodeId: n["id"] as String,
          failureDomain: n["domain"] as String,
          isAcPowered: n["ac"] as bool,
          batteryPct: n["bat"] as int,
          shardHash: "blake3:c1${(i + 5) * 13}e4f5",
          sizeKb: 256,
          status: ShardDurabilityStatus.healthy,
        ));
      }
    } else if (mode == RedundancyMode.rs63) {
      for (int i = 0; i < 6; i++) {
        final n = _nodes[i];
        list.add(ShardModel(
          index: i,
          tag: "D$i",
          isParity: false,
          assignedNode: n["name"] as String,
          nodeId: n["id"] as String,
          failureDomain: n["domain"] as String,
          isAcPowered: n["ac"] as bool,
          batteryPct: n["bat"] as int,
          shardHash: "blake3:4a${(i + 1) * 7}f8e9",
          sizeKb: 171,
          status: ShardDurabilityStatus.healthy,
        ));
      }
      for (int i = 0; i < 3; i++) {
        final n = _nodes[6 + i];
        list.add(ShardModel(
          index: 6 + i,
          tag: "P$i",
          isParity: true,
          assignedNode: n["name"] as String,
          nodeId: n["id"] as String,
          failureDomain: n["domain"] as String,
          isAcPowered: n["ac"] as bool,
          batteryPct: n["bat"] as int,
          shardHash: "blake3:9d${(i + 7) * 9}a1b2",
          sizeKb: 171,
          status: ShardDurabilityStatus.healthy,
        ));
      }
    } else {
      for (int i = 0; i < 3; i++) {
        final n = _nodes[i];
        list.add(ShardModel(
          index: i,
          tag: "R${i + 1}",
          isParity: i > 0,
          assignedNode: n["name"] as String,
          nodeId: n["id"] as String,
          failureDomain: n["domain"] as String,
          isAcPowered: n["ac"] as bool,
          batteryPct: n["bat"] as int,
          shardHash: "blake3:e3b0c442",
          sizeKb: 1024,
          status: ShardDurabilityStatus.healthy,
        ));
      }
    }
    return list;
  }

  void switchMode(RedundancyMode mode) {
    state = state.copyWith(
      mode: mode,
      shards: _buildShards(mode),
      repairLogs: const [],
      isHealing: false,
    );
  }

  void simulateOutage() {
    if (state.isHealing) return;
    final dropCount = state.mode == RedundancyMode.rep3 ? 1 : 2;
    final updated = List<ShardModel>.from(state.shards);
    for (int i = 0; i < dropCount; i++) {
      if (i < updated.length) {
        updated[i] = updated[i].copyWith(status: ShardDurabilityStatus.offline);
      }
    }
    final now = DateTime.now().toIso8601String().substring(11, 19);
    final log = "[$now] ⚠️ Simulated outage: Dropped shards ${updated.take(dropCount).map((s) => s.tag).join(', ')}.";
    state = state.copyWith(
      shards: updated,
      repairLogs: [log, ...state.repairLogs],
    );
  }

  Future<void> selfHeal() async {
    if (state.offlineShards == 0 || state.isHealing) return;
    state = state.copyWith(isHealing: true);

    final now = DateTime.now().toIso8601String().substring(11, 19);
    final initLog = "[$now] ⚙️ Reconstructing missing shards via GF(2^8) Cauchy matrix inversion...";
    state = state.copyWith(repairLogs: [initLog, ...state.repairLogs]);

    await Future<void>.delayed(const Duration(milliseconds: 500));

    final updated = state.shards.map((s) {
      if (s.status == ShardDurabilityStatus.offline) {
        final spare = _nodes[6];
        return s.copyWith(
          assignedNode: spare["name"] as String,
          nodeId: spare["id"] as String,
          failureDomain: spare["domain"] as String,
          isAcPowered: spare["ac"] as bool,
          batteryPct: spare["bat"] as int,
          status: ShardDurabilityStatus.healthy,
        );
      }
      return s;
    }).toList();

    final endLog = "[$now] ✅ Durability restored: Reconstructed shards migrated to spare nodes.";
    state = state.copyWith(
      shards: updated,
      isHealing: false,
      repairLogs: [endLog, ...state.repairLogs],
    );
  }

  void reset() {
    state = _initialState();
  }
}

final erasureMatrixProvider =
    StateNotifierProvider<ErasureMatrixNotifier, ErasureMatrixState>((ref) {
  return ErasureMatrixNotifier();
});

class ErasureShardMatrixScreen extends ConsumerWidget {
  const ErasureShardMatrixScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(erasureMatrixProvider);
    final notifier = ref.read(erasureMatrixProvider.notifier);

    return Scaffold(
      backgroundColor: const Color(0xFF0F172A),
      appBar: AppBar(
        title: const Text("Erasure Shard Placement Matrix"),
        backgroundColor: const Color(0xFF1E293B),
        elevation: 0,
      ),
      body: LayoutBuilder(
        builder: (context, constraints) {
          final isCompact = constraints.maxWidth < 600;
          return SingleChildScrollView(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _buildDurabilityHeader(state),
                const SizedBox(height: 16),
                _buildProfileSelector(state, notifier),
                const SizedBox(height: 16),
                _buildCryptoInfoCard(),
                const SizedBox(height: 16),
                _buildShardGrid(state, isCompact),
                const SizedBox(height: 20),
                _buildControlsBar(state, notifier),
                if (state.repairLogs.isNotEmpty) ...[
                  const SizedBox(height: 20),
                  _buildTimelineLogs(state),
                ],
              ],
            ),
          );
        },
      ),
    );
  }

  Widget _buildDurabilityHeader(ErasureMatrixState state) {
    Color bg;
    Color border;
    Color text;
    String label;

    if (state.isUnrecoverable) {
      bg = const Color(0x26EF4444);
      border = const Color(0xFFEF4444);
      text = const Color(0xFFF87171);
      label = "Unrecoverable (${state.healthyShards}/${state.totalShards} Shards)";
    } else if (state.isDegraded) {
      bg = const Color(0x26F59E0B);
      border = const Color(0xFFF59E0B);
      text = const Color(0xFFFBBF24);
      label = "Degraded (${state.healthyShards}/${state.totalShards} Shards) — Tolerates ${state.toleratesOutages} Outages";
    } else {
      bg = const Color(0x2610B981);
      border = const Color(0xFF10B981);
      text = const Color(0xFF34D399);
      label = "${state.healthyShards}/${state.totalShards} Shards Healthy — Tolerates ${state.toleratesOutages} Node Outages";
    }

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      decoration: BoxDecoration(
        color: bg,
        border: Border.all(color: border),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        children: [
          Icon(Icons.shield_outlined, color: text, size: 20),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              label,
              style: TextStyle(color: text, fontWeight: FontWeight.bold, fontSize: 14),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildProfileSelector(ErasureMatrixState state, ErasureMatrixNotifier notifier) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text(
          "Redundancy Profile",
          style: TextStyle(color: Color(0xFF94A3B8), fontSize: 13, fontWeight: FontWeight.w600),
        ),
        const SizedBox(height: 8),
        SegmentedButton<RedundancyMode>(
          segments: const [
            ButtonSegment(
              value: RedundancyMode.rs42,
              label: Text("RS(4, 2) Balanced"),
            ),
            ButtonSegment(
              value: RedundancyMode.rs63,
              label: Text("RS(6, 3) High"),
            ),
            ButtonSegment(
              value: RedundancyMode.rep3,
              label: Text("3x Replicated"),
            ),
          ],
          selected: {state.mode},
          onSelectionChanged: (set) => notifier.switchMode(set.first),
          style: ButtonStyle(
            backgroundColor: WidgetStateProperty.resolveWith((states) {
              if (states.contains(WidgetState.selected)) {
                return const Color(0xFF0284C7);
              }
              return const Color(0xFF1E293B);
            }),
            foregroundColor: const WidgetStatePropertyAll(Colors.white),
          ),
        ),
      ],
    );
  }

  Widget _buildCryptoInfoCard() {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFF0A0F1D),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: const Color(0xFF1E293B)),
      ),
      child: const Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            "🔒 Client-Side Convergent Encryption: XChaCha20-Poly1305 (24-byte Nonce)",
            style: TextStyle(color: Color(0xFFCBD5E1), fontSize: 12),
          ),
          SizedBox(height: 4),
          Text(
            "🔑 HKDF-SHA256(namespace_secret, blake3(chunk)) &bull; Shannon Entropy: 7.9984 bits/byte",
            style: TextStyle(color: Color(0xFF34D399), fontSize: 12),
          ),
        ],
      ),
    );
  }

  Widget _buildShardGrid(ErasureMatrixState state, bool isCompact) {
    final crossAxisCount = isCompact ? 1 : 2;
    return GridView.builder(
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: crossAxisCount,
        mainAxisExtent: 140,
        crossAxisSpacing: 12,
        mainAxisSpacing: 12,
      ),
      itemCount: state.shards.length,
      itemBuilder: (context, index) {
        final shard = state.shards[index];
        return _buildShardTile(shard);
      },
    );
  }

  Widget _buildShardTile(ShardModel shard) {
    final isOffline = shard.status == ShardDurabilityStatus.offline;
    final borderColor = isOffline
        ? const Color(0xFFEF4444)
        : shard.isParity
            ? const Color(0xFF6366F1)
            : const Color(0xFF0284C7);

    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFF1E293B),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: borderColor, width: 1.5),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Row(
            children: [
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: shard.isParity ? const Color(0xFF6366F1) : const Color(0xFF0284C7),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  shard.tag,
                  style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 12),
                ),
              ),
              const SizedBox(width: 8),
              Text(
                shard.isParity ? "Parity Shard" : "Data Shard",
                style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 11),
              ),
              const Spacer(),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: isOffline ? const Color(0x26EF4444) : const Color(0x2610B981),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  shard.status.name.toUpperCase(),
                  style: TextStyle(
                    color: isOffline ? const Color(0xFFF87171) : const Color(0xFF34D399),
                    fontSize: 10,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ),
            ],
          ),
          Row(
            children: [
              const Icon(Icons.computer, size: 14, color: Color(0xFF64748B)),
              const SizedBox(width: 6),
              Expanded(
                child: Text(
                  shard.assignedNode,
                  style: const TextStyle(color: Colors.white, fontSize: 12, fontFamily: 'monospace'),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
          Row(
            children: [
              Text(
                shard.failureDomain,
                style: const TextStyle(color: Color(0xFF38BDF8), fontSize: 11),
              ),
              const Spacer(),
              Text(
                shard.isAcPowered ? "⚡ AC" : "🔋 ${shard.batteryPct}%",
                style: const TextStyle(color: Color(0xFFFBBF24), fontSize: 11),
              ),
            ],
          ),
          Text(
            "${shard.shardHash} &bull; ${shard.sizeKb} KiB",
            style: const TextStyle(color: Color(0xFF64748B), fontSize: 10, fontFamily: 'monospace'),
          ),
        ],
      ),
    );
  }

  Widget _buildControlsBar(ErasureMatrixState state, ErasureMatrixNotifier notifier) {
    return Wrap(
      spacing: 12,
      runSpacing: 12,
      children: [
        ElevatedButton.icon(
          onPressed: (state.offlineShards > 0 || state.isHealing) ? null : notifier.simulateOutage,
          icon: const Icon(Icons.power_off, size: 16),
          label: const Text("Simulate Node Outage"),
          style: ElevatedButton.styleFrom(
            backgroundColor: const Color(0xFF991B1B),
            foregroundColor: Colors.white,
          ),
        ),
        ElevatedButton.icon(
          onPressed: (state.offlineShards == 0 || state.isHealing) ? null : notifier.selfHeal,
          icon: const Icon(Icons.healing, size: 16),
          label: Text(state.isHealing ? "Reconstructing..." : "Reconstruct & Self-Heal"),
          style: ElevatedButton.styleFrom(
            backgroundColor: const Color(0xFF0284C7),
            foregroundColor: Colors.white,
          ),
        ),
        OutlinedButton(
          onPressed: notifier.reset,
          style: OutlinedButton.styleFrom(
            foregroundColor: const Color(0xFFCBD5E1),
            side: const BorderSide(color: Color(0xFF334155)),
          ),
          child: const Text("Reset Cluster"),
        ),
      ],
    );
  }

  Widget _buildTimelineLogs(ErasureMatrixState state) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFF0A0F1D),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: const Color(0xFF1E293B)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            "Self-Healing Repair Timeline",
            style: TextStyle(color: Color(0xFF94A3B8), fontSize: 12, fontWeight: FontWeight.bold),
          ),
          const SizedBox(height: 8),
          ...state.repairLogs.map(
            (log) => Padding(
              padding: const EdgeInsets.only(bottom: 4),
              child: Text(
                log,
                style: const TextStyle(color: Color(0xFFCBD5E1), fontSize: 11, fontFamily: 'monospace'),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

