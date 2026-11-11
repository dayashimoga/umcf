import 'package:flutter/material.dart';

/// Represents a stage in the distributed pipeline for UI visualization.
class UiLayerStage {
  final int stageIndex;
  final String nodeId;
  final String deviceName;
  final String deviceType; // 'Desktop', 'Laptop', 'Phone'
  final int startLayer;
  final int endLayer;
  final bool hasEmbeddings;
  final bool hasOutputHead;
  final double usedMemoryMb;
  final double totalMemoryMb;
  final double hopLatencyMs;

  const UiLayerStage({
    required this.stageIndex,
    required this.nodeId,
    required this.deviceName,
    required this.deviceType,
    required this.startLayer,
    required this.endLayer,
    required this.hasEmbeddings,
    required this.hasOutputHead,
    required this.usedMemoryMb,
    required this.totalMemoryMb,
    required this.hopLatencyMs,
  });

  int get layerCount => endLayer - startLayer + 1;
  double get memoryRatio => totalMemoryMb > 0 ? (usedMemoryMb / totalMemoryMb).clamp(0.0, 1.0) : 0.0;
}

/// Interactive visual ring and pipeline topology widget for distributed LLM inference.
class DistributedLayerTopologyWidget extends StatefulWidget {
  final String modelName;
  final int totalLayers;
  final double estimatedDecodeTps;
  final double estimatedTtftMs;
  final bool isSingleNode;
  final List<UiLayerStage> stages;
  final VoidCallback? onRebalanceRequested;

  const DistributedLayerTopologyWidget({
    super.key,
    required this.modelName,
    required this.totalLayers,
    required this.estimatedDecodeTps,
    required this.estimatedTtftMs,
    required this.isSingleNode,
    required this.stages,
    this.onRebalanceRequested,
  });

  @override
  State<DistributedLayerTopologyWidget> createState() => _DistributedLayerTopologyWidgetState();
}

class _DistributedLayerTopologyWidgetState extends State<DistributedLayerTopologyWidget> {
  int? _selectedStageIndex;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      elevation: 3,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      child: Padding(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header: Model Name & Estimated Performance
            Row(
              children: [
                Icon(
                  widget.isSingleNode ? Icons.computer : Icons.hub,
                  color: theme.colorScheme.primary,
                  size: 28,
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        widget.modelName,
                        style: theme.textTheme.titleMedium?.copyWith(
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      Text(
                        '${widget.totalLayers} Layers | ${widget.isSingleNode ? "Single Device" : "${widget.stages.length}-Node Pipeline Ring"}',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
                // Performance badges
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.primaryContainer,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.end,
                    children: [
                      Text(
                        '${widget.estimatedDecodeTps.toStringAsFixed(1)} tok/s',
                        style: TextStyle(
                          color: theme.colorScheme.onPrimaryContainer,
                          fontWeight: FontWeight.bold,
                          fontSize: 13,
                        ),
                      ),
                      Text(
                        'TTFT: ${widget.estimatedTtftMs.toStringAsFixed(0)} ms',
                        style: TextStyle(
                          color: theme.colorScheme.onPrimaryContainer,
                          fontSize: 11,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
            const Divider(height: 24),

            // Pipeline Stages List
            Text(
              'Layer Allocation Topology',
              style: theme.textTheme.labelLarge?.copyWith(
                fontWeight: FontWeight.bold,
              ),
            ),
            const SizedBox(height: 8),

            ListView.separated(
              shrinkWrap: true,
              physics: const NeverScrollableScrollPhysics(),
              itemCount: widget.stages.length,
              separatorBuilder: (context, index) {
                final currentStage = widget.stages[index];
                return Padding(
                  padding: const EdgeInsets.symmetric(vertical: 4.0),
                  child: Row(
                    children: [
                      const SizedBox(width: 24),
                      Icon(Icons.arrow_downward, size: 16, color: theme.colorScheme.outline),
                      const SizedBox(width: 8),
                      Text(
                        'mTLS QUIC Hop: ${currentStage.hopLatencyMs.toStringAsFixed(1)} ms',
                        style: TextStyle(
                          fontSize: 11,
                          color: theme.colorScheme.outline,
                          fontFamily: 'monospace',
                        ),
                      ),
                    ],
                  ),
                );
              },
              itemBuilder: (context, index) {
                final stage = widget.stages[index];
                final isSelected = _selectedStageIndex == index;

                return InkWell(
                  onTap: () {
                    setState(() {
                      _selectedStageIndex = isSelected ? null : index;
                    });
                  },
                  borderRadius: BorderRadius.circular(12),
                  child: Container(
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: isSelected
                          ? theme.colorScheme.surfaceContainerHighest
                          : theme.colorScheme.surface,
                      borderRadius: BorderRadius.circular(12),
                      border: Border.all(
                        color: isSelected
                            ? theme.colorScheme.primary
                            : theme.colorScheme.outlineVariant,
                        width: isSelected ? 2 : 1,
                      ),
                    ),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            _buildDeviceIcon(stage.deviceType, theme),
                            const SizedBox(width: 10),
                            Expanded(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text(
                                    stage.deviceName,
                                    style: const TextStyle(fontWeight: FontWeight.bold),
                                  ),
                                  Text(
                                    stage.nodeId,
                                    style: TextStyle(
                                      fontSize: 10,
                                      color: theme.colorScheme.onSurfaceVariant,
                                      fontFamily: 'monospace',
                                    ),
                                  ),
                                ],
                              ),
                            ),
                            // Layer badge
                            Chip(
                              label: Text(
                                'Layers ${stage.startLayer}–${stage.endLayer} (${stage.layerCount})',
                                style: const TextStyle(fontSize: 11, fontWeight: FontWeight.bold),
                              ),
                              padding: EdgeInsets.zero,
                              materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                              backgroundColor: theme.colorScheme.secondaryContainer,
                            ),
                          ],
                        ),
                        const SizedBox(height: 8),

                        // Special role badges
                        Row(
                          children: [
                            if (stage.hasEmbeddings) ...[
                              _buildRoleBadge('Embeddings', Colors.teal, theme),
                              const SizedBox(width: 6),
                            ],
                            if (stage.hasOutputHead) ...[
                              _buildRoleBadge('Output Head', Colors.indigo, theme),
                              const SizedBox(width: 6),
                            ],
                            const Spacer(),
                            Text(
                              '${(stage.usedMemoryMb / 1024.0).toStringAsFixed(1)} / ${(stage.totalMemoryMb / 1024.0).toStringAsFixed(1)} GB',
                              style: TextStyle(fontSize: 11, color: theme.colorScheme.onSurfaceVariant),
                            ),
                          ],
                        ),
                        const SizedBox(height: 6),

                        // VRAM / RAM Usage Indicator
                        ClipRRect(
                          borderRadius: BorderRadius.circular(4),
                          child: LinearProgressIndicator(
                            value: stage.memoryRatio,
                            minHeight: 6,
                            backgroundColor: theme.colorScheme.outlineVariant,
                            valueColor: AlwaysStoppedAnimation<Color>(
                              stage.memoryRatio > 0.85
                                  ? Colors.red
                                  : stage.memoryRatio > 0.70
                                      ? Colors.orange
                                      : Colors.green,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                );
              },
            ),

            const SizedBox(height: 16),
            // Rebalance Action
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                OutlinedButton.icon(
                  onPressed: widget.onRebalanceRequested,
                  icon: const Icon(Icons.refresh, size: 18),
                  label: const Text('Rebalance Partition Plan'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildDeviceIcon(String deviceType, ThemeData theme) {
    IconData icon;
    switch (deviceType.toLowerCase()) {
      case 'desktop':
        icon = Icons.desktop_windows;
        break;
      case 'laptop':
        icon = Icons.laptop;
        break;
      case 'phone':
        icon = Icons.phone_android;
        break;
      default:
        icon = Icons.devices;
    }
    return CircleAvatar(
      radius: 18,
      backgroundColor: theme.colorScheme.primaryContainer,
      child: Icon(icon, size: 20, color: theme.colorScheme.onPrimaryContainer),
    );
  }

  Widget _buildRoleBadge(String label, Color color, ThemeData theme) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: color.withOpacity(0.15),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: color.withOpacity(0.4)),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: color,
          fontSize: 10,
          fontWeight: FontWeight.bold,
        ),
      ),
    );
  }
}

