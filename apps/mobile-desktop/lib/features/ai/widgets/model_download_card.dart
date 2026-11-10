// Flutter widget for model download status, bitfield heatmap, and transfer controls.
// Conforms to UMCF Rule 02: Zero non-functional UI controls.

import 'package:flutter/material.dart';

enum DownloadUiStatus { downloading, paused, completed, failed, cancelled }

class ModelDownloadState {
  final String modelId;
  final String filename;
  final int totalSizeBytes;
  final int downloadedBytes;
  final int verifiedChunks;
  final int totalChunks;
  final List<bool> bitfield;
  final double bytesPerSec;
  final double? etaSeconds;
  final int activePeers;
  final DownloadUiStatus status;

  const ModelDownloadState({
    required this.modelId,
    required this.filename,
    required this.totalSizeBytes,
    required this.downloadedBytes,
    required this.verifiedChunks,
    required this.totalChunks,
    required this.bitfield,
    required this.bytesPerSec,
    this.etaSeconds,
    required this.activePeers,
    required this.status,
  });

  double get progressRatio =>
      totalSizeBytes > 0 ? (downloadedBytes / totalSizeBytes).clamp(0.0, 1.0) : 0.0;

  String get formattedSpeed {
    final mbps = bytesPerSec / (1024 * 1024);
    return '${mbps.toStringAsFixed(1)} MB/s';
  }

  String get formattedSize {
    final mb = totalSizeBytes / (1024 * 1024);
    final downloadedMb = downloadedBytes / (1024 * 1024);
    return '${downloadedMb.toStringAsFixed(1)} / ${mb.toStringAsFixed(1)} MB';
  }
}

class ModelDownloadCard extends StatelessWidget {
  final ModelDownloadState state;
  final VoidCallback onPause;
  final VoidCallback onResume;
  final VoidCallback onCancel;

  const ModelDownloadCard({
    super.key,
    required this.state,
    required this.onPause,
    required this.onResume,
    required this.onCancel,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      elevation: 3,
      margin: const EdgeInsets.symmetric(vertical: 8, horizontal: 16),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      child: Padding(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.memory, color: Colors.indigo, size: 28),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        state.filename,
                        style: const TextStyle(
                          fontSize: 16,
                          fontWeight: FontWeight.bold,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                      Text(
                        'ID: ${state.modelId.substring(0, 12)}...',
                        style: TextStyle(fontSize: 12, color: Colors.grey.shade600),
                      ),
                    ],
                  ),
                ),
                _buildStatusChip(state.status),
              ],
            ),
            const SizedBox(height: 14),
            LinearProgressIndicator(
              value: state.progressRatio,
              backgroundColor: Colors.grey.shade200,
              color: state.status == DownloadUiStatus.paused
                  ? Colors.amber
                  : Colors.indigo,
              minHeight: 8,
              borderRadius: BorderRadius.circular(4),
            ),
            const SizedBox(height: 8),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  '${(state.progressRatio * 100).toStringAsFixed(1)}% (${state.formattedSize})',
                  style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w500),
                ),
                Text(
                  '${state.verifiedChunks} / ${state.totalChunks} chunks',
                  style: TextStyle(fontSize: 12, color: Colors.grey.shade700),
                ),
              ],
            ),
            const SizedBox(height: 12),
            _buildBitfieldHeatmap(state.bitfield),
            const SizedBox(height: 12),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Row(
                  children: [
                    const Icon(Icons.speed, size: 16, color: Colors.blueGrey),
                    const SizedBox(width: 4),
                    Text(state.formattedSpeed, style: const TextStyle(fontSize: 12)),
                    const SizedBox(width: 16),
                    const Icon(Icons.hub, size: 16, color: Colors.blueGrey),
                    const SizedBox(width: 4),
                    Text('${state.activePeers} peers', style: const TextStyle(fontSize: 12)),
                  ],
                ),
                Row(
                  children: [
                    if (state.status == DownloadUiStatus.downloading)
                      IconButton(
                        icon: const Icon(Icons.pause_circle_filled, color: Colors.amber),
                        tooltip: 'Pause Download',
                        onPressed: onPause,
                      )
                    else if (state.status == DownloadUiStatus.paused)
                      IconButton(
                        icon: const Icon(Icons.play_circle_filled, color: Colors.indigo),
                        tooltip: 'Resume Download',
                        onPressed: onResume,
                      ),
                    IconButton(
                      icon: const Icon(Icons.cancel, color: Colors.redAccent),
                      tooltip: 'Cancel Download',
                      onPressed: onCancel,
                    ),
                  ],
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildStatusChip(DownloadUiStatus status) {
    Color chipColor;
    String label;
    switch (status) {
      case DownloadUiStatus.downloading:
        chipColor = Colors.green;
        label = 'Downloading';
        break;
      case DownloadUiStatus.paused:
        chipColor = Colors.amber;
        label = 'Paused';
        break;
      case DownloadUiStatus.completed:
        chipColor = Colors.blue;
        label = 'Completed';
        break;
      case DownloadUiStatus.failed:
        chipColor = Colors.red;
        label = 'Failed';
        break;
      case DownloadUiStatus.cancelled:
        chipColor = Colors.grey;
        label = 'Cancelled';
        break;
    }
    return Chip(
      label: Text(
        label,
        style: const TextStyle(color: Colors.white, fontSize: 11, fontWeight: FontWeight.bold),
      ),
      backgroundColor: chipColor,
      visualDensity: VisualDensity.compact,
      padding: EdgeInsets.zero,
    );
  }

  Widget _buildBitfieldHeatmap(List<bool> bitfield) {
    if (bitfield.isEmpty) {
      return const SizedBox.shrink();
    }
    final displayCount = bitfield.length > 128 ? 128 : bitfield.length;
    return Container(
      padding: const EdgeInsets.all(6),
      decoration: BoxDecoration(
        color: Colors.grey.shade100,
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: Colors.grey.shade300),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text('Chunk Bitfield Map', style: TextStyle(fontSize: 10, fontWeight: FontWeight.bold)),
              Text('${bitfield.length} chunks (4 MiB)', style: const TextStyle(fontSize: 10, color: Colors.grey)),
            ],
          ),
          const SizedBox(height: 6),
          Wrap(
            spacing: 2,
            runSpacing: 2,
            children: List.generate(displayCount, (index) {
              final isDone = bitfield[index];
              return Container(
                width: 7,
                height: 7,
                decoration: BoxDecoration(
                  color: isDone ? Colors.green.shade600 : Colors.grey.shade300,
                  borderRadius: BorderRadius.circular(1),
                ),
              );
            }),
          ),
        ],
      ),
    );
  }
}

