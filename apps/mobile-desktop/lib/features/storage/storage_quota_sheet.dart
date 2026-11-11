import 'package:flutter/material.dart';

/// Storage allocation breakdown record received from Rust storage engine.
class StorageBreakdown {
  final double pinnedGb;
  final double modelShardsGb;
  final double cachedChunksGb;
  final double inFlightReservedMb;
  final double hostFreeGb;
  final double hostSafetyFloorGb;
  final double configuredMaxGb;

  const StorageBreakdown({
    required this.pinnedGb,
    required this.modelShardsGb,
    required this.cachedChunksGb,
    required this.inFlightReservedMb,
    required this.hostFreeGb,
    required this.hostSafetyFloorGb,
    required this.configuredMaxGb,
  });

  factory StorageBreakdown.initial(double maxGb) {
    return StorageBreakdown(
      pinnedGb: 4.2,
      modelShardsGb: 12.8,
      cachedChunksGb: 6.4,
      inFlightReservedMb: 800.0,
      hostFreeGb: 45.0,
      hostSafetyFloorGb: 5.0,
      configuredMaxGb: maxGb,
    );
  }

  double get totalUsedGb =>
      pinnedGb + modelShardsGb + cachedChunksGb + (inFlightReservedMb / 1024.0);

  double get usedPercentage =>
      ((totalUsedGb / configuredMaxGb) * 100).clamp(0.0, 100.0);
}

/// Flutter Modal Sheet for managing Content-Addressed Storage Quota & Cache (UMCF S7.P1.SP1).
class StorageQuotaSheet extends StatefulWidget {
  final String nodeId;
  final double initialMaxGb;
  final void Function(double newMaxGb)? onQuotaChanged;
  final Future<double> Function()? onPurgeCacheRequested;

  const StorageQuotaSheet({
    super.key,
    this.nodeId = 'local-storage-node',
    this.initialMaxGb = 32.0,
    this.onQuotaChanged,
    this.onPurgeCacheRequested,
  });

  @override
  State<StorageQuotaSheet> createState() => _StorageQuotaSheetState();
}

class _StorageQuotaSheetState extends State<StorageQuotaSheet> {
  late double _configuredMaxGb;
  late StorageBreakdown _breakdown;
  bool _isPurging = false;

  @override
  void initState() {
    super.initState();
    _configuredMaxGb = widget.initialMaxGb;
    _breakdown = StorageBreakdown.initial(widget.initialMaxGb);
  }

  Future<void> _handlePurgeCache() async {
    setState(() => _isPurging = true);
    try {
      double freedGb = _breakdown.cachedChunksGb;
      if (widget.onPurgeCacheRequested != null) {
        freedGb = await widget.onPurgeCacheRequested!();
      } else {
        await Future.delayed(const Duration(milliseconds: 300));
        setState(() {
          _breakdown = StorageBreakdown(
            pinnedGb: _breakdown.pinnedGb,
            modelShardsGb: _breakdown.modelShardsGb,
            cachedChunksGb: 0.0,
            inFlightReservedMb: _breakdown.inFlightReservedMb,
            hostFreeGb: _breakdown.hostFreeGb + freedGb,
            hostSafetyFloorGb: _breakdown.hostSafetyFloorGb,
            configuredMaxGb: _configuredMaxGb,
          );
        });
      }
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            backgroundColor: const Color(0xFF10B981),
            content: Text('✨ Purged ${freedGb.toStringAsFixed(2)} GB unpinned chunk cache'),
            duration: const Duration(seconds: 2),
          ),
        );
      }
    } finally {
      if (mounted) {
        setState(() => _isPurging = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: const BoxDecoration(
        color: Color(0xFF0F172A),
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Row(
                children: [
                  const Icon(Icons.storage, color: Color(0xFF38BDF8), size: 24),
                  const SizedBox(width: 10),
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text(
                        'Content-Addressed Storage Quota',
                        style: TextStyle(
                          fontSize: 16,
                          fontWeight: FontWeight.bold,
                          color: Color(0xFFF1F5F9),
                        ),
                      ),
                      Text(
                        'Node: ${widget.nodeId} · FastCDC + BLAKE3',
                        style: const TextStyle(fontSize: 11, color: Color(0xFF94A3B8)),
                      ),
                    ],
                  ),
                ],
              ),
              OutlinedButton.icon(
                icon: const Icon(Icons.cleaning_services, size: 14, color: Color(0xFFCBD5E1)),
                label: Text(
                  _isPurging ? 'Purging…' : 'Purge Cache',
                  style: const TextStyle(fontSize: 12, color: Color(0xFFCBD5E1)),
                ),
                onPressed: _breakdown.cachedChunksGb > 0 && !_isPurging ? _handlePurgeCache : null,
                style: OutlinedButton.styleFrom(
                  side: const BorderSide(color: Color(0xFF334155)),
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                ),
              ),
            ],
          ),
          const SizedBox(height: 18),

          // Allocation Progress Section
          Container(
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: const Color(0xFF0B0F19),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: const Color(0xFF1E293B)),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(
                      '${_breakdown.totalUsedGb.toStringAsFixed(2)} GB used of ${_configuredMaxGb.toInt()} GB (${_breakdown.usedPercentage.toStringAsFixed(1)}%)',
                      style: const TextStyle(
                        fontSize: 12,
                        fontWeight: FontWeight.bold,
                        color: Color(0xFFE2E8F0),
                      ),
                    ),
                    Text(
                      'Host Free: ${_breakdown.hostFreeGb.toStringAsFixed(1)} GB (Floor: ${_breakdown.hostSafetyFloorGb.toStringAsFixed(1)} GB)',
                      style: const TextStyle(
                        fontSize: 11,
                        fontFamily: 'monospace',
                        color: Color(0xFF94A3B8),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 10),

                // Stacked Bar
                ClipRRect(
                  borderRadius: BorderRadius.circular(6),
                  child: SizedBox(
                    height: 12,
                    child: Row(
                      children: [
                        if (_breakdown.pinnedGb > 0)
                          Expanded(
                            flex: (_breakdown.pinnedGb * 100).toInt(),
                            child: Container(color: const Color(0xFF10B981)),
                          ),
                        if (_breakdown.modelShardsGb > 0)
                          Expanded(
                            flex: (_breakdown.modelShardsGb * 100).toInt(),
                            child: Container(color: const Color(0xFF3B82F6)),
                          ),
                        if (_breakdown.cachedChunksGb > 0)
                          Expanded(
                            flex: (_breakdown.cachedChunksGb * 100).toInt(),
                            child: Container(color: const Color(0xFF6366F1)),
                          ),
                        if (_breakdown.inFlightReservedMb > 0)
                          Expanded(
                            flex: ((_breakdown.inFlightReservedMb / 1024.0) * 100).toInt(),
                            child: Container(color: const Color(0xFFF59E0B)),
                          ),
                        Expanded(
                          flex: ((_configuredMaxGb - _breakdown.totalUsedGb).clamp(0.0, _configuredMaxGb) * 100).toInt(),
                          child: Container(color: const Color(0xFF1E293B)),
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(height: 12),

                // Legend
                Wrap(
                  spacing: 12,
                  runSpacing: 6,
                  children: [
                    _legendItem('Pinned (${_breakdown.pinnedGb.toStringAsFixed(1)} GB)', const Color(0xFF10B981)),
                    _legendItem('Models (${_breakdown.modelShardsGb.toStringAsFixed(1)} GB)', const Color(0xFF3B82F6)),
                    _legendItem('Cache (${_breakdown.cachedChunksGb.toStringAsFixed(1)} GB)', const Color(0xFF6366F1)),
                    _legendItem('In-Flight (${_breakdown.inFlightReservedMb.toInt()} MB)', const Color(0xFFF59E0B)),
                    _legendItem('Host Floor (15%)', const Color(0xFFEF4444)),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 18),

          // Quota Slider Section
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text(
                'Configured Storage Cap',
                style: TextStyle(fontSize: 12, fontWeight: FontWeight.bold, color: Color(0xFFCBD5E1)),
              ),
              Text(
                '${_configuredMaxGb.toInt()} GB',
                style: const TextStyle(
                  fontFamily: 'monospace',
                  fontSize: 14,
                  fontWeight: FontWeight.bold,
                  color: Color(0xFF38BDF8),
                ),
              ),
            ],
          ),
          Slider(
            value: _configuredMaxGb,
            min: 1.0,
            max: 128.0,
            divisions: 127,
            activeColor: const Color(0xFF38BDF8),
            onChanged: (val) {
              setState(() {
                _configuredMaxGb = val;
                _breakdown = StorageBreakdown(
                  pinnedGb: _breakdown.pinnedGb,
                  modelShardsGb: _breakdown.modelShardsGb,
                  cachedChunksGb: _breakdown.cachedChunksGb,
                  inFlightReservedMb: _breakdown.inFlightReservedMb,
                  hostFreeGb: _breakdown.hostFreeGb,
                  hostSafetyFloorGb: _breakdown.hostSafetyFloorGb,
                  configuredMaxGb: val,
                );
              });
              widget.onQuotaChanged?.call(val);
            },
          ),
          const Padding(
            padding: EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text('1 GB', style: TextStyle(fontSize: 10, color: Color(0xFF64748B), fontFamily: 'monospace')),
                Text('32 GB', style: TextStyle(fontSize: 10, color: Color(0xFF64748B), fontFamily: 'monospace')),
                Text('64 GB', style: TextStyle(fontSize: 10, color: Color(0xFF64748B), fontFamily: 'monospace')),
                Text('128 GB', style: TextStyle(fontSize: 10, color: Color(0xFF64748B), fontFamily: 'monospace')),
              ],
            ),
          ),
          const SizedBox(height: 12),
        ],
      ),
    );
  }

  Widget _legendItem(String label, Color color) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          width: 8,
          height: 8,
          decoration: BoxDecoration(color: color, shape: BoxShape.circle),
        ),
        const SizedBox(width: 4),
        Text(
          label,
          style: const TextStyle(fontSize: 10, color: Color(0xFFCBD5E1)),
        ),
      ],
    );
  }
}

