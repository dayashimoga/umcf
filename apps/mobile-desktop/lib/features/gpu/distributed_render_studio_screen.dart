import 'package:flutter/material.dart';

/// Worker node contribution in distributed rendering session.
class WorkerNodeContribution {
  final String workerId;
  final String name;
  final Color color;
  final int completedTiles;
  final double relativeTflops;

  const WorkerNodeContribution({
    required this.workerId,
    required this.name,
    required this.color,
    required this.completedTiles,
    required this.relativeTflops,
  });
}

/// Flutter Screen for Progressive Distributed Rendering Studio (UMCF S6.P1.SP2).
/// Supports desktop, tablet, and mobile adaptive breakpoints with interactive tile attribution.
class DistributedRenderStudioScreen extends StatefulWidget {
  final List<WorkerNodeContribution> workers;
  final int totalTiles;
  final Size resolution;

  const DistributedRenderStudioScreen({
    super.key,
    this.workers = const [
      WorkerNodeContribution(
        workerId: 'w-rtx4090',
        name: 'Desktop RTX 4090',
        color: Colors.blueAccent,
        completedTiles: 72,
        relativeTflops: 12.0,
      ),
      WorkerNodeContribution(
        workerId: 'w-m3pro',
        name: 'MacBook M3 Pro',
        color: Colors.greenAccent,
        completedTiles: 36,
        relativeTflops: 4.0,
      ),
      WorkerNodeContribution(
        workerId: 'w-ipad',
        name: 'iPad Pro M2',
        color: Colors.purpleAccent,
        completedTiles: 18,
        relativeTflops: 2.0,
      ),
      WorkerNodeContribution(
        workerId: 'w-pixel8',
        name: 'Pixel 8 Pro',
        color: Colors.orangeAccent,
        completedTiles: 9,
        relativeTflops: 1.0,
      ),
    ],
    this.totalTiles = 135,
    this.resolution = const Size(1920, 1080),
  });

  @override
  State<DistributedRenderStudioScreen> createState() => _DistributedRenderStudioScreenState();
}

class _DistributedRenderStudioScreenState extends State<DistributedRenderStudioScreen> {
  bool _showAttributionOverlay = true;
  bool _isRendering = false;
  String? _selectedWorkerId;

  int get _completedTiles => widget.workers.fold(0, (sum, w) => sum + w.completedTiles);
  double get _progress => widget.totalTiles > 0 ? (_completedTiles / widget.totalTiles).clamp(0.0, 1.0) : 0.0;

  void _triggerRender() {
    setState(() => _isRendering = true);
    Future.delayed(const Duration(milliseconds: 1500), () {
      if (mounted) {
        setState(() => _isRendering = false);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Distributed Render Studio'),
        actions: [
          IconButton(
            tooltip: _showAttributionOverlay ? 'Hide Attribution Overlay' : 'Show Attribution Overlay',
            icon: Icon(
              _showAttributionOverlay ? Icons.layers : Icons.layers_clear,
              color: _showAttributionOverlay ? Colors.lightBlueAccent : Colors.grey,
            ),
            onPressed: () {
              setState(() {
                _showAttributionOverlay = !_showAttributionOverlay;
              });
            },
          ),
        ],
      ),
      body: LayoutBuilder(
        builder: (context, constraints) {
          final isCompact = constraints.maxWidth < 720;
          return Padding(
            padding: const EdgeInsets.all(16.0),
            child: isCompact ? _buildCompactLayout() : _buildExpandedLayout(),
          );
        },
      ),
    );
  }

  Widget _buildExpandedLayout() {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Expanded(
          flex: 3,
          child: _buildCanvasArea(),
        ),
        const SizedBox(width: 16),
        SizedBox(
          width: 320,
          child: _buildSidebar(),
        ),
      ],
    );
  }

  Widget _buildCompactLayout() {
    return SingleChildScrollView(
      child: Column(
        children: [
          _buildCanvasArea(),
          const SizedBox(height: 16),
          _buildSidebar(),
        ],
      ),
    );
  }

  Widget _buildCanvasArea() {
    return Card(
      elevation: 3,
      color: const Color(0xFF14141A),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      child: Padding(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  'Master Frame (${widget.resolution.width.toInt()} × ${widget.resolution.height.toInt()})',
                  style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 16),
                ),
                Chip(
                  label: Text('Progress: ${(_progress * 100).toStringAsFixed(1)}%'),
                  backgroundColor: Colors.blue.withAlpha(50),
                ),
              ],
            ),
            const SizedBox(height: 12),
            AspectRatio(
              aspectRatio: widget.resolution.width / widget.resolution.height,
              child: Container(
                decoration: BoxDecoration(
                  color: const Color(0xFF0D0D12),
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(color: Colors.white12),
                ),
                child: CustomPaint(
                  painter: _TileGridPainter(
                    workers: widget.workers,
                    totalTiles: widget.totalTiles,
                    showAttribution: _showAttributionOverlay,
                    selectedWorkerId: _selectedWorkerId,
                  ),
                ),
              ),
            ),
            const SizedBox(height: 12),
            ElevatedButton.icon(
              onPressed: _isRendering ? null : _triggerRender,
              icon: _isRendering
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                    )
                  : const Icon(Icons.play_arrow),
              label: Text(_isRendering ? 'Rendering Distributed Tiles...' : 'Start Distributed Render'),
              style: ElevatedButton.styleFrom(
                backgroundColor: Colors.teal,
                foregroundColor: Colors.white,
                minimumSize: const Size.fromHeight(44),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildSidebar() {
    return Card(
      elevation: 2,
      color: const Color(0xFF181822),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      child: Padding(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              'Worker Node Contribution',
              style: TextStyle(fontWeight: FontWeight.bold, fontSize: 15),
            ),
            const SizedBox(height: 12),
            ...widget.workers.map((worker) => _buildWorkerRow(worker)),
            const Divider(color: Colors.white12, height: 24),
            const Text(
              'Cluster Efficiency',
              style: TextStyle(fontWeight: FontWeight.bold, fontSize: 14),
            ),
            const SizedBox(height: 6),
            const Text(
              '3.64× linear speedup (92.4% ideal)',
              style: TextStyle(color: Colors.greenAccent, fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 4),
            const Text(
              'Speculative tail execution: 1 straggler mitigated',
              style: TextStyle(color: Colors.white70, fontSize: 12),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildWorkerRow(WorkerNodeContribution worker) {
    final sharePercent = (_completedTiles > 0)
        ? (worker.completedTiles / _completedTiles * 100).toStringAsFixed(0)
        : '0';

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6.0),
      child: InkWell(
        onTap: () {
          setState(() {
            _selectedWorkerId = (_selectedWorkerId == worker.workerId) ? null : worker.workerId;
          });
        },
        borderRadius: BorderRadius.circular(4),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Container(
                  width: 10,
                  height: 10,
                  decoration: BoxDecoration(color: worker.color, shape: BoxShape.circle),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(worker.name, style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w500)),
                ),
                Text('${worker.relativeTflops} TF', style: const TextStyle(fontSize: 11, color: Colors.grey)),
              ],
            ),
            const SizedBox(height: 4),
            LinearProgressIndicator(
              value: widget.totalTiles > 0 ? worker.completedTiles / widget.totalTiles : 0,
              backgroundColor: Colors.white10,
              valueColor: AlwaysStoppedAnimation<Color>(worker.color),
              minHeight: 4,
            ),
            const SizedBox(height: 2),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text('${worker.completedTiles} tiles', style: const TextStyle(fontSize: 11, color: Colors.white60)),
                Text('$sharePercent% share', style: const TextStyle(fontSize: 11, color: Colors.white60)),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _TileGridPainter extends CustomPainter {
  final List<WorkerNodeContribution> workers;
  final int totalTiles;
  final bool showAttribution;
  final String? selectedWorkerId;

  _TileGridPainter({
    required this.workers,
    required this.totalTiles,
    required this.showAttribution,
    required this.selectedWorkerId,
  });

  @override
  void paint(Canvas canvas, Size size) {
    const cols = 15;
    const rows = 9;
    final tileW = size.width / cols;
    final tileH = size.height / rows;

    final borderPaint = Paint()
      ..color = Colors.black26
      ..style = PaintingStyle.stroke
      ..strokeWidth = 0.5;

    int tileIdx = 0;
    for (int r = 0; r < rows; r++) {
      for (int c = 0; c < cols; c++) {
        if (tileIdx >= totalTiles) break;

        final rect = Rect.fromLTWH(c * tileW, r * tileH, tileW, tileH);
        final worker = workers[tileIdx % workers.length];

        final isSelected = selectedWorkerId == null || selectedWorkerId == worker.workerId;
        final baseColor = showAttribution ? worker.color.withAlpha(isSelected ? 100 : 30) : const Color(0xFF22222E);

        final fillPaint = Paint()..color = baseColor;
        canvas.drawRect(rect, fillPaint);
        canvas.drawRect(rect, borderPaint);

        if (showAttribution && isSelected) {
          final outlinePaint = Paint()
            ..color = worker.color
            ..style = PaintingStyle.stroke
            ..strokeWidth = 1.0;
          canvas.drawRect(rect, outlinePaint);
        }

        tileIdx++;
      }
    }
  }

  @override
  bool shouldRepaint(covariant _TileGridPainter oldDelegate) {
    return oldDelegate.showAttribution != showAttribution ||
        oldDelegate.selectedWorkerId != selectedWorkerId;
  }
}
