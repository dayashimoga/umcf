import 'dart:async';
import 'package:flutter/material.dart';

/// Telemetry metrics snapshot received from the Rust transport engine over FFI / stream sink.
class RemoteStreamTelemetry {
  final int streamId;
  final int frameSeq;
  final double glassToGlassLatencyMs;
  final double encodeLatencyMs;
  final double networkRttMs;
  final double jitterMs;
  final double packetLossPct;
  final double fecRecoveryPct;
  final double bitrateMbps;
  final double targetBitrateMbps;
  final String resolution;
  final int fps;
  final String abrState; // 'RampUp', 'Hold', 'Backoff', 'ResolutionDownshift'
  final double delayGradientMs;
  final int nackCount;
  final int pliCount;

  const RemoteStreamTelemetry({
    required this.streamId,
    required this.frameSeq,
    required this.glassToGlassLatencyMs,
    required this.encodeLatencyMs,
    required this.networkRttMs,
    required this.jitterMs,
    required this.packetLossPct,
    required this.fecRecoveryPct,
    required this.bitrateMbps,
    required this.targetBitrateMbps,
    required this.resolution,
    required this.fps,
    required this.abrState,
    required this.delayGradientMs,
    required this.nackCount,
    required this.pliCount,
  });

  factory RemoteStreamTelemetry.initial(int streamId, double initialBitrate) {
    return RemoteStreamTelemetry(
      streamId: streamId,
      frameSeq: 12050,
      glassToGlassLatencyMs: 16.8,
      encodeLatencyMs: 2.2,
      networkRttMs: 8.4,
      jitterMs: 0.9,
      packetLossPct: 1.1,
      fecRecoveryPct: 99.5,
      bitrateMbps: initialBitrate,
      targetBitrateMbps: initialBitrate,
      resolution: '1920 × 1080',
      fps: 60,
      abrState: 'Hold',
      delayGradientMs: 0.28,
      nackCount: 12,
      pliCount: 1,
    );
  }
}

/// Remote Application & Game Streaming Client View (UMCF S6.P3.SP2).
/// Displays live video viewport with adaptive HUD overlay and latency telemetry.
class RemoteStreamClientView extends StatefulWidget {
  final int streamId;
  final String hostNodeName;
  final String videoCodec;
  final double initialBitrateMbps;
  final void Function(int streamId)? onKeyframeRequested;
  final void Function(double capMbps)? onBitrateCapChanged;

  const RemoteStreamClientView({
    super.key,
    this.streamId = 1,
    this.hostNodeName = 'Gaming-Host-Desktop',
    this.videoCodec = 'H.264 Baseline/High (Zero-B)',
    this.initialBitrateMbps = 25.0,
    this.onKeyframeRequested,
    this.onBitrateCapChanged,
  });

  @override
  State<RemoteStreamClientView> createState() => _RemoteStreamClientViewState();
}

class _RemoteStreamClientViewState extends State<RemoteStreamClientView> {
  late RemoteStreamTelemetry _telemetry;
  bool _isHudVisible = true;
  bool _isCompactHud = false;
  late double _selectedBitrateCap;
  bool _autoAbrEnabled = true;
  Timer? _telemetryTicker;

  @override
  void initState() {
    super.initState();
    _selectedBitrateCap = widget.initialBitrateMbps;
    _telemetry = RemoteStreamTelemetry.initial(widget.streamId, widget.initialBitrateMbps);

    // Live telemetry ticker simulating FFI updates
    _telemetryTicker = Timer.periodic(const Duration(milliseconds: 150), (timer) {
      if (!mounted) return;
      setState(() {
        final newSeq = _telemetry.frameSeq + 1;
        final rttDelta = (newSeq % 5 == 0) ? 0.3 : -0.2;
        final newRtt = (_telemetry.networkRttMs + rttDelta).clamp(4.0, 35.0);
        final newJitter = (0.7 + ((newSeq % 7) * 0.05)).clamp(0.2, 5.0);
        final newG2g = 5.2 + _telemetry.encodeLatencyMs + newRtt + newJitter;

        _telemetry = RemoteStreamTelemetry(
          streamId: _telemetry.streamId,
          frameSeq: newSeq,
          glassToGlassLatencyMs: double.parse(newG2g.toStringAsFixed(1)),
          encodeLatencyMs: _telemetry.encodeLatencyMs,
          networkRttMs: double.parse(newRtt.toStringAsFixed(1)),
          jitterMs: double.parse(newJitter.toStringAsFixed(2)),
          packetLossPct: _telemetry.packetLossPct,
          fecRecoveryPct: _telemetry.fecRecoveryPct,
          bitrateMbps: _telemetry.bitrateMbps,
          targetBitrateMbps: _selectedBitrateCap,
          resolution: _telemetry.resolution,
          fps: _telemetry.fps,
          abrState: _telemetry.abrState,
          delayGradientMs: _telemetry.delayGradientMs,
          nackCount: _telemetry.nackCount,
          pliCount: _telemetry.pliCount,
        );
      });
    });
  }

  @override
  void dispose() {
    _telemetryTicker?.cancel();
    super.dispose();
  }

  Color _getLatencyColor(double latencyMs) {
    if (latencyMs < 20.0) return const Color(0xFF10B981); // Emerald
    if (latencyMs < 35.0) return const Color(0xFFF59E0B); // Amber
    return const Color(0xFFEF4444); // Red
  }

  Color _getAbrStateColor(String state) {
    switch (state) {
      case 'Hold':
      case 'RampUp':
        return const Color(0xFF10B981);
      case 'Backoff':
        return const Color(0xFFF59E0B);
      case 'ResolutionDownshift':
        return const Color(0xFFEF4444);
      default:
        return const Color(0xFF6B7280);
    }
  }

  void _requestPliKeyframe() {
    setState(() {
      _telemetry = RemoteStreamTelemetry(
        streamId: _telemetry.streamId,
        frameSeq: _telemetry.frameSeq,
        glassToGlassLatencyMs: _telemetry.glassToGlassLatencyMs,
        encodeLatencyMs: _telemetry.encodeLatencyMs,
        networkRttMs: _telemetry.networkRttMs,
        jitterMs: _telemetry.jitterMs,
        packetLossPct: _telemetry.packetLossPct,
        fecRecoveryPct: _telemetry.fecRecoveryPct,
        bitrateMbps: _telemetry.bitrateMbps,
        targetBitrateMbps: _telemetry.targetBitrateMbps,
        resolution: _telemetry.resolution,
        fps: _telemetry.fps,
        abrState: _telemetry.abrState,
        delayGradientMs: _telemetry.delayGradientMs,
        nackCount: _telemetry.nackCount,
        pliCount: _telemetry.pliCount + 1,
      );
    });
    widget.onKeyframeRequested?.call(widget.streamId);
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('IDR Keyframe Requested (PLI packet sent)'),
        duration: Duration(seconds: 1),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isDesktop = MediaQuery.of(context).size.width > 900;

    return Scaffold(
      backgroundColor: const Color(0xFF030712),
      appBar: AppBar(
        title: Row(
          children: [
            const Icon(Icons.cast_connected, color: Color(0xFF38BDF8), size: 20),
            const SizedBox(width: 8),
            Text(
              widget.hostNodeName,
              style: const TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
            ),
            const SizedBox(width: 12),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
              decoration: BoxDecoration(
                color: const Color(0xFF0E7490).withAlpha(80),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: const Color(0xFF0E7490)),
              ),
              child: Text(
                '${_telemetry.fps} FPS · ${_telemetry.resolution}',
                style: const TextStyle(fontSize: 11, color: Color(0xFF38BDF8)),
              ),
            ),
          ],
        ),
        backgroundColor: const Color(0xFF0F172A),
        actions: [
          IconButton(
            icon: Icon(_isHudVisible ? Icons.analytics : Icons.analytics_outlined),
            tooltip: _isHudVisible ? 'Hide Telemetry HUD' : 'Show Telemetry HUD',
            onPressed: () => setState(() => _isHudVisible = !_isHudVisible),
          ),
          IconButton(
            icon: const Icon(Icons.refresh, color: Color(0xFFFCA5A5)),
            tooltip: 'Request Keyframe (PLI)',
            onPressed: _requestPliKeyframe,
          ),
        ],
      ),
      body: Stack(
        children: [
          // Simulated Low-Latency Video Viewport Surface
          Center(
            child: AspectRatio(
              aspectRatio: 16 / 9,
              child: Container(
                decoration: BoxDecoration(
                  color: const Color(0xFF0B0F19),
                  border: Border.all(color: const Color(0xFF1F2937)),
                  gradient: const RadialGradient(
                    center: Alignment.center,
                    radius: 0.9,
                    colors: [Color(0xFF1E293B), Color(0xFF030712)],
                  ),
                ),
                child: Stack(
                  children: [
                    // Watermark Barcode & Presentation Timestamp indicator
                    Positioned(
                      bottom: 16,
                      right: 16,
                      child: Container(
                        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                        decoration: BoxDecoration(
                          color: Colors.black.withAlpha(180),
                          borderRadius: BorderRadius.circular(6),
                          border: Border.all(color: const Color(0xFF374151)),
                        ),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.end,
                          children: [
                            const Text(
                              '█ ▄ █ ▌ █ █ ▌ ▄',
                              style: TextStyle(
                                fontFamily: 'monospace',
                                color: Color(0xFF10B981),
                                fontSize: 13,
                                letterSpacing: 2.0,
                              ),
                            ),
                            Text(
                              'PTS: ${(_telemetry.frameSeq * 16666)} µs',
                              style: const TextStyle(
                                fontFamily: 'monospace',
                                color: Color(0xFF9CA3AF),
                                fontSize: 10,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                    Center(
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(
                            Icons.sports_esports,
                            size: 64,
                            color: Colors.white.withAlpha(40),
                          ),
                          const SizedBox(height: 8),
                          Text(
                            'Active Low-Latency Session (${widget.videoCodec})',
                            style: TextStyle(
                              color: Colors.white.withAlpha(120),
                              fontSize: 13,
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),

          // Floating Telemetry HUD Overlay
          if (_isHudVisible)
            Positioned(
              top: 16,
              right: 16,
              child: _buildTelemetryHud(isDesktop),
            ),

          // Bottom Control Dock
          Positioned(
            bottom: 0,
            left: 0,
            right: 0,
            child: _buildBottomToolbar(),
          ),
        ],
      ),
    );
  }

  Widget _buildTelemetryHud(bool isDesktop) {
    final abrColor = _getAbrStateColor(_telemetry.abrState);
    final latencyColor = _getLatencyColor(_telemetry.glassToGlassLatencyMs);

    return Container(
      width: _isCompactHud ? 220 : 310,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFF0F172A).withAlpha(235),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: const Color(0xFF334155)),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withAlpha(150),
            blurRadius: 16,
            offset: const Offset(0, 8),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Row(
                children: [
                  Container(
                    width: 8,
                    height: 8,
                    decoration: BoxDecoration(
                      color: abrColor,
                      shape: BoxShape.circle,
                    ),
                  ),
                  const SizedBox(width: 6),
                  const Text(
                    'QUIC Transport HUD',
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.bold,
                      color: Color(0xFFF1F5F9),
                    ),
                  ),
                ],
              ),
              Row(
                children: [
                  InkWell(
                    onTap: () => setState(() => _isCompactHud = !_isCompactHud),
                    child: Padding(
                      padding: const EdgeInsets.all(2),
                      child: Text(
                        _isCompactHud ? 'EXPAND' : 'COMPACT',
                        style: const TextStyle(
                          fontSize: 9,
                          fontWeight: FontWeight.bold,
                          color: Color(0xFF94A3B8),
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  InkWell(
                    onTap: () => setState(() => _isHudVisible = false),
                    child: const Icon(Icons.close, size: 14, color: Color(0xFF94A3B8)),
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 8),
          const Divider(height: 1, color: Color(0xFF334155)),
          const SizedBox(height: 8),

          // Metrics Grid
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              _buildMetricTile(
                'Glass-to-Glass',
                '${_telemetry.glassToGlassLatencyMs} ms',
                valueColor: latencyColor,
              ),
              _buildMetricTile(
                'RTT / Jitter',
                '${_telemetry.networkRttMs} / ${_telemetry.jitterMs} ms',
              ),
              _buildMetricTile(
                'Bitrate (Cap)',
                '${_telemetry.bitrateMbps} / ${_selectedBitrateCap.toInt()} Mbps',
              ),
              _buildMetricTile(
                'Loss / FEC Rec',
                '${_telemetry.packetLossPct}% / ${_telemetry.fecRecoveryPct}%',
              ),
              if (!_isCompactHud) ...[
                _buildMetricTile(
                  'ABR State',
                  _telemetry.abrState,
                  valueColor: abrColor,
                ),
                _buildMetricTile(
                  'Delay Gradient Δd',
                  '${_telemetry.delayGradientMs} ms',
                ),
                _buildMetricTile(
                  'NACK / PLI',
                  '${_telemetry.nackCount} / ${_telemetry.pliCount}',
                ),
                _buildMetricTile(
                  'Resolution',
                  _telemetry.resolution,
                ),
              ],
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildMetricTile(String label, String value, {Color? valueColor}) {
    return Container(
      width: _isCompactHud ? 94 : 138,
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      decoration: BoxDecoration(
        color: const Color(0xFF1E293B).withAlpha(160),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label.toUpperCase(),
            style: const TextStyle(
              fontSize: 8,
              color: Color(0xFF94A3B8),
              fontWeight: FontWeight.w600,
              letterSpacing: 0.5,
            ),
          ),
          const SizedBox(height: 2),
          Text(
            value,
            style: TextStyle(
              fontSize: 12,
              fontFamily: 'monospace',
              fontWeight: FontWeight.bold,
              color: valueColor ?? const Color(0xFFF8FAFC),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildBottomToolbar() {
    return Container(
      height: 54,
      padding: const EdgeInsets.symmetric(horizontal: 16),
      decoration: BoxDecoration(
        color: const Color(0xFF0F172A).withAlpha(240),
        border: const Border(
          top: BorderSide(color: Color(0xFF334155)),
        ),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Row(
            children: [
              OutlinedButton.icon(
                icon: const Icon(Icons.refresh, size: 16, color: Color(0xFFFCA5A5)),
                label: const Text(
                  'Request Keyframe',
                  style: TextStyle(fontSize: 12, color: Color(0xFFFCA5A5)),
                ),
                onPressed: _requestPliKeyframe,
                style: OutlinedButton.styleFrom(
                  side: const BorderSide(color: Color(0xFFEF4444)),
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                ),
              ),
              const SizedBox(width: 12),
              Row(
                children: [
                  Checkbox(
                    value: _autoAbrEnabled,
                    activeColor: const Color(0xFF38BDF8),
                    onChanged: (val) {
                      setState(() => _autoAbrEnabled = val ?? true);
                    },
                  ),
                  const Text(
                    'Auto-ABR',
                    style: TextStyle(color: Color(0xFFCBD5E1), fontSize: 12),
                  ),
                ],
              ),
            ],
          ),
          Row(
            children: [
              Text(
                'Cap: ${_selectedBitrateCap.toInt()} Mbps',
                style: const TextStyle(
                  fontFamily: 'monospace',
                  fontSize: 11,
                  color: Color(0xFF94A3B8),
                ),
              ),
              SizedBox(
                width: 120,
                child: Slider(
                  value: _selectedBitrateCap,
                  min: 2.0,
                  max: 50.0,
                  divisions: 24,
                  activeColor: const Color(0xFF38BDF8),
                  onChanged: (val) {
                    setState(() => _selectedBitrateCap = val);
                    widget.onBitrateCapChanged?.call(val);
                  },
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
