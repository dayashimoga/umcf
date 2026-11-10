// Flutter Modal Sheet for GGUF Metadata Inspection & Dynamic KV-Cache RAM/VRAM Calculation.
// Conforms to UMCF Rule 02: Zero non-functional UI controls.

import 'package:flutter/material.dart';

class GgufHyperparameters {
  final String architecture;
  final int blockCount;
  final int defaultContextLength;
  final int embeddingLength;
  final int headCount;
  final int headCountKv;
  final int totalWeightBytes;
  final String primaryQuantization;

  const GgufHyperparameters({
    required this.architecture,
    required this.blockCount,
    required this.defaultContextLength,
    required this.embeddingLength,
    required this.headCount,
    required this.headCountKv,
    required this.totalWeightBytes,
    required this.primaryQuantization,
  });
}

class GgufInspectorSheet extends StatefulWidget {
  final String filename;
  final String modelId;
  final GgufHyperparameters params;
  final int availableDeviceRamBytes;
  final VoidCallback onClose;

  const GgufInspectorSheet({
    super.key,
    required this.filename,
    required this.modelId,
    required this.params,
    this.availableDeviceRamBytes = 6 * 1024 * 1024 * 1024, // 6 GiB default
    required this.onClose,
  });

  @override
  State<GgufInspectorSheet> createState() => _GgufInspectorSheetState();
}

class _GgufInspectorSheetState extends State<GgufInspectorSheet> {
  late double _selectedContextLength;
  late int _batchSize;
  bool _flashAttention = true;

  @override
  void initState() {
    super.initState();
    _selectedContextLength = widget.params.defaultContextLength.toDouble();
    _batchSize = 512;
  }

  int get _computedKvCacheBytes {
    final ctx = _selectedContextLength.round();
    final headDim = widget.params.embeddingLength ~/ widget.params.headCount.clamp(1, 999);
    final nEmbdGqa = headDim * widget.params.headCountKv;
    // 2 * layers * ctx * n_embd_gqa * 2 (F16 bytes)
    return 2 * widget.params.blockCount * ctx * nEmbdGqa * 2;
  }

  int get _computedComputeScratchBytes {
    final ctx = _selectedContextLength.round();
    final ffnLen = widget.params.embeddingLength * 4;
    final ffnScratch = ffnLen * _batchSize * 4;
    final attnScratch = _flashAttention
        ? widget.params.headCount * (widget.params.embeddingLength ~/ widget.params.headCount) * _batchSize * 4
        : widget.params.headCount * (ctx ~/ 4) * _batchSize * 4;
    return ffnScratch + attnScratch + (48 * 1024 * 1024);
  }

  int get _totalMemoryRequired =>
      widget.params.totalWeightBytes + _computedKvCacheBytes + _computedComputeScratchBytes;

  double get _memoryUtilizationRatio =>
      _totalMemoryRequired / widget.availableDeviceRamBytes.clamp(1, double.maxFinite.toInt());

  String _formatBytes(int bytes) {
    final mb = bytes / (1024 * 1024);
    if (mb < 1024) {
      return '${mb.toStringAsFixed(1)} MiB';
    }
    return '${(mb / 1024).toStringAsFixed(2)} GiB';
  }

  @override
  Widget build(BuildContext context) {
    final ratio = _memoryUtilizationRatio;
    Color statusColor;
    String statusText;
    if (ratio <= 0.70) {
      statusColor = Colors.green;
      statusText = 'Fits Locally (High Headroom)';
    } else if (ratio <= 0.95) {
      statusColor = Colors.amber.shade800;
      statusText = 'Fits Locally (Tight Headroom)';
    } else {
      final nodes = (ratio).ceil();
      statusColor = Colors.red;
      statusText = 'Requires Cluster Split ($nodes Nodes)';
    }

    return Container(
      padding: const EdgeInsets.all(20),
      decoration: const BoxDecoration(
        color: Colors.white,
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      child: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        widget.filename,
                        style: const TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
                        overflow: TextOverflow.ellipsis,
                      ),
                      Text(
                        'Architecture: ${widget.params.architecture.toUpperCase()} • ${widget.params.primaryQuantization}',
                        style: TextStyle(fontSize: 12, color: Colors.grey.shade600),
                      ),
                    ],
                  ),
                ),
                IconButton(
                  icon: const Icon(Icons.close),
                  onPressed: widget.onClose,
                ),
              ],
            ),
            const Divider(height: 24),
            _buildHyperparametersGrid(),
            const SizedBox(height: 18),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                const Text('Context Window Length', style: TextStyle(fontWeight: FontWeight.w600)),
                Text(
                  '${_selectedContextLength.round()} tokens',
                  style: const TextStyle(fontWeight: FontWeight.bold, color: Colors.indigo),
                ),
              ],
            ),
            Slider(
              value: _selectedContextLength,
              min: 512,
              max: 65536,
              divisions: 127,
              activeColor: Colors.indigo,
              onChanged: (val) {
                setState(() {
                  _selectedContextLength = val;
                });
              },
            ),
            Row(
              children: [
                Checkbox(
                  value: _flashAttention,
                  activeColor: Colors.indigo,
                  onChanged: (val) {
                    setState(() {
                      _flashAttention = val ?? true;
                    });
                  },
                ),
                const Text('Enable Flash Attention (reduces scratch buffer)'),
              ],
            ),
            const SizedBox(height: 16),
            const Text('Estimated RAM/VRAM Allocation', style: TextStyle(fontWeight: FontWeight.w600)),
            const SizedBox(height: 8),
            ClipRRect(
              borderRadius: BorderRadius.circular(6),
              child: LinearProgressIndicator(
                value: ratio.clamp(0.0, 1.0),
                minHeight: 12,
                backgroundColor: Colors.grey.shade200,
                color: statusColor,
              ),
            ),
            const SizedBox(height: 8),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text('Required: ${_formatBytes(_totalMemoryRequired)}',
                    style: TextStyle(fontWeight: FontWeight.bold, color: statusColor)),
                Text('Device Available: ${_formatBytes(widget.availableDeviceRamBytes)}',
                    style: TextStyle(color: Colors.grey.shade600, fontSize: 12)),
              ],
            ),
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: statusColor.withOpacity(0.1),
                border: Border.all(color: statusColor.withOpacity(0.3)),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Row(
                children: [
                  Icon(ratio <= 0.95 ? Icons.check_circle : Icons.warning, color: statusColor),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Text(
                      statusText,
                      style: TextStyle(fontWeight: FontWeight.bold, color: statusColor),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 16),
            _buildComponentBreakdownTable(),
          ],
        ),
      ),
    );
  }

  Widget _buildHyperparametersGrid() {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Colors.grey.shade50,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Colors.grey.shade200),
      ),
      child: Column(
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              _paramTile('Layers', '${widget.params.blockCount}'),
              _paramTile('Embedding', '${widget.params.embeddingLength}'),
              _paramTile('Heads', '${widget.params.headCount} / ${widget.params.headCountKv} (KV)'),
            ],
          ),
        ],
      ),
    );
  }

  Widget _paramTile(String label, String value) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label, style: const TextStyle(fontSize: 11, color: Colors.grey)),
        Text(value, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.bold)),
      ],
    );
  }

  Widget _buildComponentBreakdownTable() {
    return Table(
      border: TableBorder(
        horizontalInside: BorderSide(color: Colors.grey.shade200),
      ),
      children: [
        TableRow(children: [
          const Padding(padding: EdgeInsets.all(6), child: Text('Static Model Weights')),
          Padding(
            padding: const EdgeInsets.all(6),
            child: Text(_formatBytes(widget.params.totalWeightBytes), textAlign: TextAlign.right),
          ),
        ]),
        TableRow(children: [
          const Padding(padding: EdgeInsets.all(6), child: Text('KV-Cache (F16)')),
          Padding(
            padding: const EdgeInsets.all(6),
            child: Text(_formatBytes(_computedKvCacheBytes), textAlign: TextAlign.right),
          ),
        ]),
        TableRow(children: [
          const Padding(padding: EdgeInsets.all(6), child: Text('Compute Scratch')),
          Padding(
            padding: const EdgeInsets.all(6),
            child: Text(_formatBytes(_computedComputeScratchBytes), textAlign: TextAlign.right),
          ),
        ]),
      ],
    );
  }
}
