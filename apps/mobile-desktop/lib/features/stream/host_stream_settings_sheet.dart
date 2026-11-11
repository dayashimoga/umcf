import 'package:flutter/material.dart';

/// Descriptor for a selectable display monitor or application window.
class StreamCaptureSource {
  final String id;
  final String name;
  final String type; // 'Monitor', 'Window', 'Virtual'
  final String resolution;
  final int refreshRateHz;
  final Color previewColor;

  const StreamCaptureSource({
    required this.id,
    required this.name,
    required this.type,
    required this.resolution,
    required this.refreshRateHz,
    required this.previewColor,
  });
}

/// Flutter BottomSheet / Dialog for configuring Host Low-Latency Display & Game Streaming (UMCF S6.P3.SP1).
class HostStreamSettingsSheet extends StatefulWidget {
  final String hostNodeId;
  final List<StreamCaptureSource> sources;
  final Function(Map<String, dynamic>)? onSettingsApplied;

  const HostStreamSettingsSheet({
    super.key,
    this.hostNodeId = 'local-desktop-host',
    this.sources = const [
      StreamCaptureSource(
        id: 'monitor-0',
        name: 'Primary Monitor (4K UHD)',
        type: 'Monitor',
        resolution: '3840 × 2160',
        refreshRateHz: 120,
        previewColor: Color(0xFF1E3A8A),
      ),
      StreamCaptureSource(
        id: 'monitor-1',
        name: 'Secondary Display (QHD)',
        type: 'Monitor',
        resolution: '2560 × 1440',
        refreshRateHz: 60,
        previewColor: Color(0xFF374151),
      ),
      StreamCaptureSource(
        id: 'window-game',
        name: 'DirectX 12 Game Window',
        type: 'Window',
        resolution: '2560 × 1440',
        refreshRateHz: 120,
        previewColor: Color(0xFF991B1B),
      ),
      StreamCaptureSource(
        id: 'virtual-headless',
        name: 'Headless Zero-Copy Virtual Display',
        type: 'Virtual',
        resolution: '1920 × 1080',
        refreshRateHz: 60,
        previewColor: Color(0xFF4C1D95),
      ),
    ],
    this.onSettingsApplied,
  });

  @override
  State<HostStreamSettingsSheet> createState() => _HostStreamSettingsSheetState();
}

class _HostStreamSettingsSheetState extends State<HostStreamSettingsSheet> {
  late String _selectedSourceId;
  String _selectedCodec = 'Auto (AV1 Hardware Accelerated)';
  int _targetFps = 60;
  double _bitrateMbps = 15.0;
  bool _hideNotifications = true;
  bool _blankHostScreen = false;
  bool _isStreaming = false;

  @override
  void initState() {
    super.initState();
    _selectedSourceId = widget.sources.first.id;
  }

  void _toggleStreaming() {
    setState(() {
      _isStreaming = !_isStreaming;
    });

    if (widget.onSettingsApplied != null) {
      widget.onSettingsApplied!({
        'sourceId': _selectedSourceId,
        'codec': _selectedCodec,
        'fps': _targetFps,
        'bitrateMbps': _bitrateMbps,
        'hideNotifications': _hideNotifications,
        'blankHostScreen': _blankHostScreen,
        'isStreaming': _isStreaming,
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(20.0),
      decoration: const BoxDecoration(
        color: Color(0xFF111827),
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      child: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            _buildHeader(),
            const SizedBox(height: 16),
            if (_isStreaming) ...[
              _buildLiveTelemetryBadge(),
              const SizedBox(height: 16),
            ],
            const Text(
              'Select Capture Surface or Window',
              style: TextStyle(fontSize: 14, fontWeight: FontWeight.bold, color: Colors.white),
            ),
            const SizedBox(height: 10),
            _buildSourcesList(),
            const SizedBox(height: 20),
            _buildCodecSelector(),
            const SizedBox(height: 16),
            _buildFpsSelector(),
            const SizedBox(height: 16),
            _buildBitrateSlider(),
            const SizedBox(height: 16),
            _buildPrivacyToggles(),
            const SizedBox(height: 20),
            _buildActionButton(),
          ],
        ),
      ),
    );
  }

  Widget _buildHeader() {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              'HOST STREAMING SETTINGS',
              style: TextStyle(fontSize: 11, color: Color(0xFF9CA3AF), letterSpacing: 0.5),
            ),
            const SizedBox(height: 4),
            Text(
              widget.hostNodeId,
              style: const TextStyle(fontSize: 18, fontWeight: FontWeight.bold, color: Colors.white),
            ),
          ],
        ),
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
          decoration: BoxDecoration(
            color: const Color(0xFF1E3A8A),
            borderRadius: BorderRadius.circular(4),
          ),
          child: const Text(
            'DXGI / ZERO-COPY',
            style: TextStyle(fontSize: 10, fontWeight: FontWeight.bold, color: Color(0xFF93C5FD)),
          ),
        ),
      ],
    );
  }

  Widget _buildLiveTelemetryBadge() {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: const Color(0xFF064E3B),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: const Color(0xFF059669)),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          const Row(
            children: [
              Icon(Icons.bolt, size: 16, color: Color(0xFF34D399)),
              SizedBox(width: 6),
              Text(
                'Latency: 3.2 ms (p50)',
                style: TextStyle(fontSize: 12, fontWeight: FontWeight.bold, color: Color(0xFF34D399)),
              ),
            ],
          ),
          Text(
            '59.9 / $_targetFps FPS (0 B-Frames)',
            style: const TextStyle(fontSize: 12, fontWeight: FontWeight.bold, color: Color(0xFFA7F3D0)),
          ),
        ],
      ),
    );
  }

  Widget _buildSourcesList() {
    return SizedBox(
      height: 110,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        itemCount: widget.sources.length,
        separatorBuilder: (_, __) => const SizedBox(width: 10),
        itemBuilder: (context, idx) {
          final src = widget.sources[idx];
          final isSelected = src.id == _selectedSourceId;

          return InkWell(
            onTap: () {
              setState(() {
                _selectedSourceId = src.id;
              });
            },
            borderRadius: BorderRadius.circular(8),
            child: Container(
              width: 170,
              padding: const EdgeInsets.all(8),
              decoration: BoxDecoration(
                color: const Color(0xFF1F2937),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(
                  color: isSelected ? const Color(0xFF3B82F6) : const Color(0xFF374151),
                  width: isSelected ? 2 : 1,
                ),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Container(
                    height: 44,
                    decoration: BoxDecoration(
                      color: src.previewColor,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    alignment: Alignment.center,
                    child: Text(
                      src.type.toUpperCase(),
                      style: const TextStyle(fontSize: 10, fontWeight: FontWeight.bold, color: Colors.white),
                    ),
                  ),
                  const SizedBox(height: 6),
                  Text(
                    src.name,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(fontSize: 11, fontWeight: FontWeight.bold, color: Colors.white),
                  ),
                  Text(
                    '${src.resolution} @ ${src.refreshRateHz}Hz',
                    style: const TextStyle(fontSize: 10, color: Color(0xFF9CA3AF)),
                  ),
                ],
              ),
            ),
          );
        },
      ),
    );
  }

  Widget _buildCodecSelector() {
    final codecs = [
      'Auto (AV1 Hardware Accelerated)',
      'AV1 (Sub-5ms Ultra-Low Delay)',
      'HEVC (10-bit HDR BT.2020)',
      'H.264 (Maximum Compatibility)',
    ];

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text('Video Codec Engine', style: TextStyle(fontSize: 13, color: Color(0xFFD1D5DB))),
        const SizedBox(height: 6),
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          decoration: BoxDecoration(
            color: const Color(0xFF1F2937),
            borderRadius: BorderRadius.circular(6),
            border: Border.all(color: const Color(0xFF374151)),
          ),
          child: DropdownButtonHideUnderline(
            child: DropdownButton<String>(
              isExpanded: true,
              value: _selectedCodec,
              dropdownColor: const Color(0xFF1F2937),
              style: const TextStyle(color: Colors.white, fontSize: 13),
              items: codecs.map((c) => DropdownMenuItem(value: c, child: Text(c))).toList(),
              onChanged: (val) {
                if (val != null) setState(() => _selectedCodec = val);
              },
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildFpsSelector() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text('Target Frame Rate (FPS)', style: TextStyle(fontSize: 13, color: Color(0xFFD1D5DB))),
        const SizedBox(height: 6),
        Row(
          children: [30, 60, 90, 120].map((fps) {
            final isSelected = _targetFps == fps;
            return Expanded(
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 4.0),
                child: ElevatedButton(
                  style: ElevatedButton.styleFrom(
                    backgroundColor: isSelected ? const Color(0xFF2563EB) : const Color(0xFF1F2937),
                    foregroundColor: isSelected ? Colors.white : const Color(0xFF9CA3AF),
                    padding: const EdgeInsets.symmetric(vertical: 8),
                  ),
                  onPressed: () => setState(() => _targetFps = fps),
                  child: Text('$fps FPS', style: const TextStyle(fontSize: 12, fontWeight: FontWeight.bold)),
                ),
              ),
            );
          }).toList(),
        ),
      ],
    );
  }

  Widget _buildBitrateSlider() {
    final vbvSingleFrame = ((_bitrateMbps * 1000 / _targetFps) * 1.2).round();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            const Text('Target Bitrate', style: TextStyle(fontSize: 13, color: Color(0xFFD1D5DB))),
            Text(
              '${_bitrateMbps.round()} Mbps (VBV: $vbvSingleFrame kb)',
              style: const TextStyle(fontSize: 13, fontWeight: FontWeight.bold, color: Color(0xFF60A5FA)),
            ),
          ],
        ),
        Slider(
          value: _bitrateMbps,
          min: 2,
          max: 50,
          divisions: 48,
          activeColor: const Color(0xFF3B82F6),
          inactiveColor: const Color(0xFF374151),
          onChanged: (v) => setState(() => _bitrateMbps = v),
        ),
      ],
    );
  }

  Widget _buildPrivacyToggles() {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFF1F2937),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        children: [
          SwitchListTile(
            contentPadding: EdgeInsets.zero,
            title: const Text('Hide Sensitive Notifications', style: TextStyle(fontSize: 13, color: Colors.white)),
            subtitle: const Text('Suppresses host banners during active stream', style: TextStyle(fontSize: 11, color: Color(0xFF9CA3AF))),
            value: _hideNotifications,
            activeColor: const Color(0xFF3B82F6),
            onChanged: (v) => setState(() => _hideNotifications = v),
          ),
          const Divider(color: Color(0xFF374151)),
          SwitchListTile(
            contentPadding: EdgeInsets.zero,
            title: const Text('Blank Physical Host Screen', style: TextStyle(fontSize: 13, color: Colors.white)),
            subtitle: const Text('Turns off host display for private remote sessions', style: TextStyle(fontSize: 11, color: Color(0xFF9CA3AF))),
            value: _blankHostScreen,
            activeColor: const Color(0xFF3B82F6),
            onChanged: (v) => setState(() => _blankHostScreen = v),
          ),
        ],
      ),
    );
  }

  Widget _buildActionButton() {
    return SizedBox(
      width: double.infinity,
      child: ElevatedButton.icon(
        style: ElevatedButton.styleFrom(
          backgroundColor: _isStreaming ? const Color(0xFFDC2626) : const Color(0xFF2563EB),
          foregroundColor: Colors.white,
          padding: const EdgeInsets.symmetric(vertical: 14),
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        ),
        icon: Icon(_isStreaming ? Icons.stop : Icons.play_arrow),
        label: Text(
          _isStreaming ? 'Stop Host Streaming' : 'Start Host Streaming',
          style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 14),
        ),
        onPressed: _toggleStreaming,
      ),
    );
  }
}
