import 'package:flutter/material.dart';

/// Descriptor for a GOP segment assigned to a transcode worker.
class GopSegmentItem {
  final int segmentIndex;
  final double startSec;
  final double endSec;
  final int totalFrames;
  final int transcodedFrames;
  final String assignedWorker;
  final Color nodeColor;
  final String encoderBackend;
  final double currentFps;

  const GopSegmentItem({
    required this.segmentIndex,
    required this.startSec,
    required this.endSec,
    required this.totalFrames,
    required this.transcodedFrames,
    required this.assignedWorker,
    required this.nodeColor,
    required this.encoderBackend,
    required this.currentFps,
  });

  double get progress => totalFrames > 0 ? transcodedFrames / totalFrames : 0.0;
  bool get isCompleted => transcodedFrames >= totalFrames;
}

/// Node contributing to the media transcode cluster.
class TranscodeWorkerNode {
  final String id;
  final String name;
  final String backend;
  final double fps;
  final Color color;
  final int segmentsAssigned;

  const TranscodeWorkerNode({
    required this.id,
    required this.name,
    required this.backend,
    required this.fps,
    required this.color,
    required this.segmentsAssigned,
  });
}

/// Flutter Multiplatform Screen for Segment-Parallel Transcoding Queue (UMCF S6.P2.SP2).
/// Displays visual GOP timeline, per-node FPS attribution, aggregate speedup, and sample-accurate stitching.
class TranscodeQueueScreen extends StatefulWidget {
  final String jobId;
  final String filename;
  final double durationSec;
  final int totalFrames;
  final List<GopSegmentItem> initialSegments;
  final List<TranscodeWorkerNode> initialWorkers;

  const TranscodeQueueScreen({
    super.key,
    this.jobId = 'JOB-4K60-001',
    this.filename = '4k60_master_prores.mov',
    this.durationSec = 30.0,
    this.totalFrames = 1800,
    this.initialSegments = const [
      GopSegmentItem(
        segmentIndex: 0,
        startSec: 0.0,
        endSec: 5.0,
        totalFrames: 300,
        transcodedFrames: 300,
        assignedWorker: 'Desktop RTX 4090',
        nodeColor: Color(0xFF10B981),
        encoderBackend: 'h264_nvenc',
        currentFps: 245.0,
      ),
      GopSegmentItem(
        segmentIndex: 1,
        startSec: 5.0,
        endSec: 10.0,
        totalFrames: 300,
        transcodedFrames: 300,
        assignedWorker: 'Desktop RTX 4090',
        nodeColor: Color(0xFF10B981),
        encoderBackend: 'h264_nvenc',
        currentFps: 248.0,
      ),
      GopSegmentItem(
        segmentIndex: 2,
        startSec: 10.0,
        endSec: 15.0,
        totalFrames: 300,
        transcodedFrames: 280,
        assignedWorker: 'MacBook M2 Pro',
        nodeColor: Color(0xFF3B82F6),
        encoderBackend: 'h264_videotoolbox',
        currentFps: 165.0,
      ),
      GopSegmentItem(
        segmentIndex: 3,
        startSec: 15.0,
        endSec: 20.0,
        totalFrames: 300,
        transcodedFrames: 260,
        assignedWorker: 'MacBook M2 Pro',
        nodeColor: Color(0xFF3B82F6),
        encoderBackend: 'h264_videotoolbox',
        currentFps: 162.0,
      ),
      GopSegmentItem(
        segmentIndex: 4,
        startSec: 20.0,
        endSec: 25.0,
        totalFrames: 300,
        transcodedFrames: 240,
        assignedWorker: 'Intel Core Ultra QSV',
        nodeColor: Color(0xFF8B5CF6),
        encoderBackend: 'h264_qsv',
        currentFps: 140.0,
      ),
      GopSegmentItem(
        segmentIndex: 5,
        startSec: 25.0,
        endSec: 30.0,
        totalFrames: 300,
        transcodedFrames: 190,
        assignedWorker: 'Server Threadripper',
        nodeColor: Color(0xFFF59E0B),
        encoderBackend: 'libx264',
        currentFps: 68.0,
      ),
    ],
    this.initialWorkers = const [
      TranscodeWorkerNode(
        id: 'node-nvenc',
        name: 'Desktop RTX 4090',
        backend: 'NVENC (h264_nvenc)',
        fps: 246.5,
        color: Color(0xFF10B981),
        segmentsAssigned: 2,
      ),
      TranscodeWorkerNode(
        id: 'node-vt',
        name: 'MacBook M2 Pro',
        backend: 'VideoToolbox (h264_videotoolbox)',
        fps: 163.5,
        color: Color(0xFF3B82F6),
        segmentsAssigned: 2,
      ),
      TranscodeWorkerNode(
        id: 'node-qsv',
        name: 'Intel Core Ultra QSV',
        backend: 'QuickSync (h264_qsv)',
        fps: 140.0,
        color: Color(0xFF8B5CF6),
        segmentsAssigned: 1,
      ),
      TranscodeWorkerNode(
        id: 'node-cpu',
        name: 'Server Threadripper',
        backend: 'Software CPU (libx264)',
        fps: 68.0,
        color: Color(0xFFF59E0B),
        segmentsAssigned: 1,
      ),
    ],
  });

  @override
  State<TranscodeQueueScreen> createState() => _TranscodeQueueScreenState();
}

class _TranscodeQueueScreenState extends State<TranscodeQueueScreen> {
  late List<GopSegmentItem> _segments;
  late List<TranscodeWorkerNode> _workers;
  bool _isPaused = false;
  GopSegmentItem? _selectedSegment;

  @override
  void initState() {
    super.initState();
    _segments = List.from(widget.initialSegments);
    _workers = List.from(widget.initialWorkers);
  }

  int get _totalTranscodedFrames =>
      _segments.fold(0, (acc, s) => acc + s.transcodedFrames);

  double get _aggregateFps =>
      _workers.fold(0.0, (acc, w) => acc + w.fps);

  double get _speedupFactor => _aggregateFps / 60.0;

  void _togglePause() {
    setState(() {
      _isPaused = !_isPaused;
    });
  }

  void _showSegmentDetails(GopSegmentItem seg) {
    setState(() {
      _selectedSegment = seg;
    });
    showModalBottomSheet(
      context: context,
      backgroundColor: const Color(0xFF1F2937),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (ctx) => Padding(
        padding: const EdgeInsets.all(20.0),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  'Segment #${seg.segmentIndex} Details',
                  style: const TextStyle(
                    fontSize: 18,
                    fontWeight: FontWeight.bold,
                    color: Color(0xFF60A5FA),
                  ),
                ),
                IconButton(
                  icon: const Icon(Icons.close, color: Colors.grey),
                  onPressed: () => Navigator.of(ctx).pop(),
                ),
              ],
            ),
            const Divider(color: Color(0xFF374151)),
            _buildDetailRow('Time Span:', '${seg.startSec}s – ${seg.endSec}s'),
            _buildDetailRow('Frames:', '${seg.transcodedFrames} / ${seg.totalFrames} (${(seg.progress * 100).toStringAsFixed(1)}%)'),
            _buildDetailRow('Assigned Node:', seg.assignedWorker, color: seg.nodeColor),
            _buildDetailRow('Codec Backend:', seg.encoderBackend),
            _buildDetailRow('Encoder Throughput:', '${seg.currentFps.toStringAsFixed(1)} FPS'),
            const SizedBox(height: 12),
          ],
        ),
      ),
    );
  }

  Widget _buildDetailRow(String label, String value, {Color? color}) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4.0),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label, style: const TextStyle(color: Color(0xFF9CA3AF))),
          Text(
            value,
            style: TextStyle(
              color: color ?? Colors.white,
              fontWeight: FontWeight.w600,
            ),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final overallProgress = widget.totalFrames > 0
        ? _totalTranscodedFrames / widget.totalFrames
        : 0.0;

    return Scaffold(
      backgroundColor: const Color(0xFF111827),
      appBar: AppBar(
        title: Text(widget.filename),
        backgroundColor: const Color(0xFF1F2937),
        elevation: 0,
        actions: [
          IconButton(
            icon: Icon(_isPaused ? Icons.play_arrow : Icons.pause),
            tooltip: _isPaused ? 'Resume Cluster' : 'Pause Cluster',
            onPressed: _togglePause,
          ),
        ],
      ),
      body: LayoutBuilder(
        builder: (context, constraints) {
          final isDesktop = constraints.maxWidth > 900;
          return SingleChildScrollView(
            padding: const EdgeInsets.all(16.0),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                _buildMetricsHeader(overallProgress),
                const SizedBox(height: 20),
                _buildTimelineBar(),
                const SizedBox(height: 24),
                Text(
                  'Cluster Encoding Nodes (${_workers.length})',
                  style: const TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.bold,
                    color: Colors.white,
                  ),
                ),
                const SizedBox(height: 12),
                if (isDesktop)
                  _buildDesktopWorkersGrid()
                else
                  _buildMobileWorkersList(),
              ],
            ),
          );
        },
      ),
    );
  }

  Widget _buildMetricsHeader(double overallProgress) {
    return Container(
      padding: const EdgeInsets.all(16.0),
      decoration: BoxDecoration(
        color: const Color(0xFF1F2937),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: const Color(0xFF374151)),
      ),
      child: Column(
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('CLUSTER THROUGHPUT', style: TextStyle(fontSize: 11, color: Color(0xFF9CA3AF), letterSpacing: 0.5)),
                  const SizedBox(height: 4),
                  Text(
                    '${_aggregateFps.round()} FPS (${_speedupFactor.toStringAsFixed(1)}x RT)',
                    style: const TextStyle(fontSize: 20, fontWeight: FontWeight.bold, color: Color(0xFF10B981)),
                  ),
                ],
              ),
              Column(
                crossAxisAlignment: CrossAxisAlignment.end,
                children: [
                  const Text('A/V SYNC DRIFT', style: TextStyle(fontSize: 11, color: Color(0xFF9CA3AF), letterSpacing: 0.5)),
                  const SizedBox(height: 4),
                  const Text(
                    '< 0.05 ms (Pass)',
                    style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold, color: Color(0xFF34D399)),
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 16),
          ClipRRect(
            borderRadius: BorderRadius.circular(4),
            child: LinearProgressIndicator(
              value: overallProgress,
              minHeight: 8,
              backgroundColor: const Color(0xFF374151),
              valueColor: const AlwaysStoppedAnimation<Color>(Color(0xFF3B82F6)),
            ),
          ),
          const SizedBox(height: 8),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                '$_totalTranscodedFrames / ${widget.totalFrames} frames (${(overallProgress * 100).toStringAsFixed(1)}%)',
                style: const TextStyle(fontSize: 12, color: Color(0xFF9CA3AF)),
              ),
              const Text('Sample-Accurate Closed GOP', style: TextStyle(fontSize: 12, color: Color(0xFF60A5FA))),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildTimelineBar() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text(
          'GOP Segment Boundary Timeline',
          style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: Color(0xFFD1D5DB)),
        ),
        const SizedBox(height: 8),
        Container(
          height: 48,
          decoration: BoxDecoration(
            color: const Color(0xFF1F2937),
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: const Color(0xFF374151)),
          ),
          child: Row(
            children: _segments.map((seg) {
              final flexVal = ((seg.endSec - seg.startSec) * 10).round().clamp(1, 1000);
              return Expanded(
                flex: flexVal,
                child: InkWell(
                  onTap: () => _showSegmentDetails(seg),
                  child: Container(
                    decoration: BoxDecoration(
                      border: Border(
                        right: const BorderSide(color: Color(0xFF111827), width: 1.5),
                        bottom: BorderSide(color: seg.nodeColor, width: 3.5),
                      ),
                    ),
                    child: Stack(
                      children: [
                        FractionallySizedBox(
                          widthFactor: seg.progress,
                          child: Container(
                            color: seg.nodeColor.withAlpha(80),
                          ),
                        ),
                        Center(
                          child: Text(
                            'S${seg.segmentIndex}',
                            style: const TextStyle(
                              fontSize: 11,
                              fontWeight: FontWeight.bold,
                              color: Colors.white,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              );
            }).toList(),
          ),
        ),
      ],
    );
  }

  Widget _buildDesktopWorkersGrid() {
    return GridView.builder(
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      itemCount: _workers.length,
      gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: 2,
        crossAxisSpacing: 12,
        mainAxisSpacing: 12,
        childAspectRatio: 3.2,
      ),
      itemBuilder: (context, idx) => _buildWorkerCard(_workers[idx]),
    );
  }

  Widget _buildMobileWorkersList() {
    return Column(
      children: _workers.map((w) => Padding(
        padding: const EdgeInsets.only(bottom: 8.0),
        child: _buildWorkerCard(w),
      )).toList(),
    );
  }

  Widget _buildWorkerCard(TranscodeWorkerNode worker) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFF1F2937),
        borderRadius: BorderRadius.circular(8),
        border: Border(left: BorderSide(color: worker.color, width: 4)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                worker.name,
                style: const TextStyle(fontWeight: FontWeight.bold, color: Colors.white, fontSize: 13),
              ),
              Text(
                '${worker.fps.round()} FPS',
                style: TextStyle(fontWeight: FontWeight.bold, color: worker.color, fontSize: 13),
              ),
            ],
          ),
          const SizedBox(height: 4),
          Text(
            worker.backend,
            style: const TextStyle(fontSize: 11, color: Color(0xFF9CA3AF), fontFamily: 'monospace'),
          ),
          const SizedBox(height: 2),
          Text(
            '${worker.segmentsAssigned} GOP Segments Assigned',
            style: const TextStyle(fontSize: 10, color: Color(0xFF6B7280)),
          ),
        ],
      ),
    );
  }
}
