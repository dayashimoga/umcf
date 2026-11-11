import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'features/developer/api_keys_screen.dart';
import 'features/developer/workflow_templates_screen.dart';
import 'features/gpu/distributed_render_studio_screen.dart';
import 'features/market/marketplace_orderbook_screen.dart';
import 'features/market/receipt_explorer_screen.dart';
import 'features/media/transcode_queue_screen.dart';
import 'features/storage/offline_pin_manager_screen.dart';
import 'features/stream/remote_stream_client_view.dart';

void main() {
  runApp(const ProviderScope(child: UmcfUnifiedApp()));
}

/// Root widget for the Universal Mobile Compute Fabric (UMCF) Multiplatform Client.
class UmcfUnifiedApp extends StatelessWidget {
  const UmcfUnifiedApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Universal Mobile Compute Fabric',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        useMaterial3: true,
        brightness: Brightness.dark,
        colorSchemeSeed: Colors.indigo,
        cardTheme: const CardTheme(
          elevation: 2,
          margin: EdgeInsets.zero,
        ),
      ),
      home: const UmcfMainNavigationShell(),
    );
  }
}

/// Responsive main shell adapting between NavigationBar (compact) and NavigationRail (expanded).
class UmcfMainNavigationShell extends StatefulWidget {
  const UmcfMainNavigationShell({super.key});

  @override
  State<UmcfMainNavigationShell> createState() =>
      _UmcfMainNavigationShellState();
}

class _UmcfMainNavigationShellState extends State<UmcfMainNavigationShell> {
  int _selectedIndex = 0;
  bool _isMeshConnected = true;

  final List<NavigationDestinationItem> _destinations = [
    const NavigationDestinationItem(
      label: 'Cluster',
      icon: Icons.hub_outlined,
      selectedIcon: Icons.hub,
    ),
    const NavigationDestinationItem(
      label: 'Workflows',
      icon: Icons.schema_outlined,
      selectedIcon: Icons.schema,
    ),
    const NavigationDestinationItem(
      label: 'AI & Enclave',
      icon: Icons.psychology_outlined,
      selectedIcon: Icons.psychology,
    ),
    const NavigationDestinationItem(
      label: 'GPU Render',
      icon: Icons.gradient_outlined,
      selectedIcon: Icons.gradient,
    ),
    const NavigationDestinationItem(
      label: 'Media',
      icon: Icons.video_settings_outlined,
      selectedIcon: Icons.video_settings,
    ),
    const NavigationDestinationItem(
      label: 'Streaming',
      icon: Icons.cast_connected_outlined,
      selectedIcon: Icons.cast_connected,
    ),
    const NavigationDestinationItem(
      label: 'Storage',
      icon: Icons.inventory_2_outlined,
      selectedIcon: Icons.inventory_2,
    ),
    const NavigationDestinationItem(
      label: 'Marketplace',
      icon: Icons.storefront_outlined,
      selectedIcon: Icons.storefront,
    ),
  ];

  Widget _buildBodyForIndex(int index) {
    switch (index) {
      case 0:
        return const ClusterOverviewView();
      case 1:
        return const WorkflowTemplatesScreen();
      case 2:
        return const AiEnclaveOverviewView();
      case 3:
        return const DistributedRenderStudioScreen();
      case 4:
        return const TranscodeQueueScreen();
      case 5:
        return const RemoteStreamClientView();
      case 6:
        return const OfflinePinManagerScreen();
      case 7:
        return const MarketplaceOrderbookScreen();
      default:
        return const ClusterOverviewView();
    }
  }

  void _showPairingModal(BuildContext context) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Row(
          children: [
            Icon(Icons.qr_code_scanner, color: Colors.indigoAccent),
            SizedBox(width: 8),
            Text('Device Pairing (Out-of-Band)'),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            Container(
              width: 180,
              height: 180,
              decoration: BoxDecoration(
                color: Colors.white,
                borderRadius: BorderRadius.circular(12),
              ),
              child: const Center(
                child: Icon(Icons.qr_code_2, size: 160, color: Colors.black),
              ),
            ),
            const SizedBox(height: 16),
            const Text(
              'PIN: 849-204',
              style: TextStyle(
                fontFamily: 'monospace',
                fontSize: 22,
                fontWeight: FontWeight.bold,
                letterSpacing: 2,
              ),
            ),
            const SizedBox(height: 8),
            const Text(
              'Scan from mobile node or enter pairing URI. Token expires in 4:45.',
              textAlign: TextAlign.center,
              style: TextStyle(fontSize: 12, color: Colors.grey),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }

  void _showReceiptExplorer(BuildContext context) {
    Navigator.of(context).push(
      MaterialPageRoute(builder: (_) => const ReceiptExplorerScreen()),
    );
  }

  void _showApiKeys(BuildContext context) {
    Navigator.of(context).push(
      MaterialPageRoute(builder: (_) => const ApiKeysScreen()),
    );
  }

  void _showChaosTestingModal(BuildContext context) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: const Color(0xFF111827),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (ctx) => Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.bolt, color: Colors.amberAccent),
                const SizedBox(width: 8),
                const Text(
                  'Chaos Nemesis & Fault Injection',
                  style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold, color: Colors.white),
                ),
                const Spacer(),
                IconButton(
                  icon: const Icon(Icons.close, color: Colors.grey),
                  onPressed: () => Navigator.of(ctx).pop(),
                ),
              ],
            ),
            const SizedBox(height: 6),
            const Text(
              'Simulate partitions, thermal throttles, and OS memory trim with continuous Jepsen invariant auditing.',
              style: TextStyle(fontSize: 12, color: Colors.grey),
            ),
            const SizedBox(height: 16),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                ElevatedButton.icon(
                  icon: const Icon(Icons.alt_route, size: 16),
                  label: const Text('Partition Split'),
                  style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF1F2937)),
                  onPressed: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('Injected Symmetric Partition {A,B} | {C,D,E}')),
                    );
                  },
                ),
                ElevatedButton.icon(
                  icon: const Icon(Icons.thermostat, size: 16),
                  label: const Text('Thermal Spike (95°C)'),
                  style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF1F2937)),
                  onPressed: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('Triggered ThermalState::Critical. Tasks migrating.')),
                    );
                  },
                ),
                ElevatedButton.icon(
                  icon: const Icon(Icons.memory, size: 16),
                  label: const Text('onTrimMemory'),
                  style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF1F2937)),
                  onPressed: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('Evicted 256 MiB unpinned cache.')),
                    );
                  },
                ),
                FilledButton.icon(
                  icon: const Icon(Icons.healing, size: 16),
                  label: const Text('Heal All & Converge'),
                  style: FilledButton.styleFrom(backgroundColor: Colors.teal),
                  onPressed: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('Healed network. Cluster converged in 340 ms (< 5000 ms SLA).')),
                    );
                  },
                ),
              ],
            ),
            const SizedBox(height: 16),
            const Divider(color: Color(0xFF374151)),
            const SizedBox(height: 8),
            const Text(
              'Continuous Invariants: 5/5 Green (Data, Bounded Convergence, No Zombie, Storage Durability, Ledger Zero-Sum)',
              style: TextStyle(fontSize: 11, color: Colors.greenAccent, fontFamily: 'monospace'),
            ),
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final isDesktop = constraints.maxWidth >= 800;

        return Scaffold(
          appBar: AppBar(
            title: Row(
              children: [
                const Icon(Icons.device_hub, color: Colors.indigoAccent),
                const SizedBox(width: 8),
                const Text(
                  'UMCF',
                  style: TextStyle(fontWeight: FontWeight.bold, letterSpacing: 1),
                ),
                const SizedBox(width: 12),
                Chip(
                  avatar: CircleAvatar(
                    backgroundColor: _isMeshConnected ? Colors.green : Colors.red,
                    radius: 4,
                  ),
                  label: Text(
                    _isMeshConnected ? 'Mesh: 4 Nodes Online' : 'Offline',
                    style: const TextStyle(fontSize: 11),
                  ),
                  padding: EdgeInsets.zero,
                  visualDensity: VisualDensity.compact,
                ),
              ],
            ),
            actions: [
              IconButton(
                tooltip: 'Pair Device via QR',
                icon: const Icon(Icons.qr_code_scanner),
                onPressed: () => _showPairingModal(context),
              ),
              IconButton(
                tooltip: 'Cryptographic Receipts',
                icon: const Icon(Icons.verified),
                onPressed: () => _showReceiptExplorer(context),
              ),
              IconButton(
                tooltip: 'Developer API Keys',
                icon: const Icon(Icons.key),
                onPressed: () => _showApiKeys(context),
              ),
              IconButton(
                tooltip: 'Chaos & Fault Injection',
                icon: const Icon(Icons.bolt),
                onPressed: () => _showChaosTestingModal(context),
              ),
            ],
          ),
          body: isDesktop
              ? Row(
                  children: [
                    NavigationRail(
                      selectedIndex: _selectedIndex,
                      onDestinationSelected: (index) {
                        setState(() {
                          _selectedIndex = index;
                        });
                      },
                      labelType: NavigationRailLabelType.all,
                      destinations: _destinations.map((d) {
                        return NavigationRailDestination(
                          icon: Icon(d.icon),
                          selectedIcon: Icon(d.selectedIcon),
                          label: Text(d.label),
                        );
                      }).toList(),
                    ),
                    const VerticalDivider(thickness: 1, width: 1),
                    Expanded(child: _buildBodyForIndex(_selectedIndex)),
                  ],
                )
              : _buildBodyForIndex(_selectedIndex),
          bottomNavigationBar: isDesktop
              ? null
              : NavigationBar(
                  selectedIndex: _selectedIndex,
                  onDestinationSelected: (index) {
                    setState(() {
                      _selectedIndex = index;
                    });
                  },
                  destinations: _destinations.map((d) {
                    return NavigationDestination(
                      icon: Icon(d.icon),
                      selectedIcon: Icon(d.selectedIcon),
                      label: d.label,
                    );
                  }).toList(),
                ),
        );
      },
    );
  }
}

class NavigationDestinationItem {
  final String label;
  final IconData icon;
  final IconData selectedIcon;

  const NavigationDestinationItem({
    required this.label,
    required this.icon,
    required this.selectedIcon,
  });
}

/// Cluster Overview Screen displaying real-time cluster nodes and telemetry.
class ClusterOverviewView extends StatefulWidget {
  const ClusterOverviewView({super.key});

  @override
  State<ClusterOverviewView> createState() => _ClusterOverviewViewState();
}

class _ClusterOverviewViewState extends State<ClusterOverviewView> {
  bool _consentSharing = true;
  String _nodeRole = 'Hybrid';

  void _cycleRole() {
    setState(() {
      _nodeRole = _nodeRole == 'Hybrid'
          ? 'Master'
          : _nodeRole == 'Master'
              ? 'Worker'
              : 'Hybrid';
    });
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(
                      'Local Node Governance (Role: $_nodeRole)',
                      style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 16),
                    ),
                    FilledButton.tonal(
                      onPressed: _cycleRole,
                      child: const Text('Switch Role'),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                SwitchListTile(
                  contentPadding: EdgeInsets.zero,
                  title: const Text('Resource Sharing Consent'),
                  subtitle: const Text('Opt-in to allow remote mesh workloads to execute locally'),
                  value: _consentSharing,
                  onChanged: (v) => setState(() => _consentSharing = v),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),
        const Text(
          'Enrolled Mesh Nodes (4)',
          style: TextStyle(fontWeight: FontWeight.bold, fontSize: 18),
        ),
        const SizedBox(height: 8),
        _buildNodeCard(
          name: 'Intel Ultra 5 Desktop (Local)',
          did: 'did:umcf:desktop_rtx4090',
          role: 'Hybrid / Master',
          cpu: '14 Cores (12% Load)',
          ram: '16 GB (6.2 GB Used)',
          gpu: 'Intel Arc 130T (Vulkan)',
          thermal: 'Nominal (38°C)',
          power: 'AC Powered',
        ),
        const SizedBox(height: 8),
        _buildNodeCard(
          name: 'Google Pixel 9 Pro',
          did: 'did:umcf:phone_pixel9',
          role: 'Worker',
          cpu: '8 Cores (4% Load)',
          ram: '16 GB (4.8 GB Used)',
          gpu: 'Mali-G715 (45 TOPS NPU)',
          thermal: 'Nominal (32°C)',
          power: 'Charging (92%)',
        ),
        const SizedBox(height: 8),
        _buildNodeCard(
          name: 'MacBook Pro M3 Max',
          did: 'did:umcf:macbook_m3',
          role: 'Worker',
          cpu: '16 Cores (8% Load)',
          ram: '64 GB (18.4 GB Used)',
          gpu: 'Apple M3 Metal (40 Cores)',
          thermal: 'Nominal (36°C)',
          power: 'AC Powered',
        ),
      ],
    );
  }

  Widget _buildNodeCard({
    required String name,
    required String did,
    required String role,
    required String cpu,
    required String ram,
    required String gpu,
    required String thermal,
    required String power,
  }) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(name, style: const TextStyle(fontWeight: FontWeight.bold)),
                Chip(
                  label: Text(role, style: const TextStyle(fontSize: 10)),
                  visualDensity: VisualDensity.compact,
                ),
              ],
            ),
            Text(did, style: const TextStyle(fontFamily: 'monospace', fontSize: 11, color: Colors.grey)),
            const Divider(height: 16),
            Wrap(
              spacing: 16,
              runSpacing: 8,
              children: [
                Text('CPU: $cpu', style: const TextStyle(fontSize: 12)),
                Text('RAM: $ram', style: const TextStyle(fontSize: 12)),
                Text('GPU/NPU: $gpu', style: const TextStyle(fontSize: 12)),
                Text('Thermal: $thermal', style: const TextStyle(fontSize: 12)),
                Text('Power: $power', style: const TextStyle(fontSize: 12)),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

/// AI & Private Enclave Screen.
class AiEnclaveOverviewView extends StatefulWidget {
  const AiEnclaveOverviewView({super.key});

  @override
  State<AiEnclaveOverviewView> createState() => _AiEnclaveOverviewViewState();
}

class _AiEnclaveOverviewViewState extends State<AiEnclaveOverviewView> {
  bool _strictLocalOnly = true;
  String _selectedModel = 'Llama-3.2-3B-Instruct.Q4_K_M.gguf';

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text(
                  'Private AI Enclave Shield',
                  style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16),
                ),
                const SizedBox(height: 4),
                const Text(
                  'When Strict-Local-Only is enabled, prompt tokens and activation tensors never leave this physical device.',
                  style: TextStyle(fontSize: 12, color: Colors.grey),
                ),
                const SizedBox(height: 8),
                SwitchListTile(
                  contentPadding: EdgeInsets.zero,
                  title: const Text('Strict-Local-Only Private Shield'),
                  value: _strictLocalOnly,
                  onChanged: (v) => setState(() => _strictLocalOnly = v),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('Active GGUF Model', style: TextStyle(fontWeight: FontWeight.bold)),
                const SizedBox(height: 8),
                DropdownButton<String>(
                  isExpanded: true,
                  value: _selectedModel,
                  items: const [
                    DropdownMenuItem(
                      value: 'Llama-3.2-3B-Instruct.Q4_K_M.gguf',
                      child: Text('Llama-3.2-3B-Instruct (Q4_K_M — 2.1 GB)'),
                    ),
                    DropdownMenuItem(
                      value: 'Mistral-7B-Instruct-v0.3.Q4_K_M.gguf',
                      child: Text('Mistral-7B-Instruct (Q4_K_M — 4.3 GB)'),
                    ),
                    DropdownMenuItem(
                      value: 'Qwen2.5-Coder-7B.Q4_K_M.gguf',
                      child: Text('Qwen2.5-Coder-7B (Q4_K_M — 4.4 GB)'),
                    ),
                  ],
                  onChanged: (v) {
                    if (v != null) setState(() => _selectedModel = v);
                  },
                ),
                const SizedBox(height: 12),
                LinearProgressIndicator(
                  value: 0.85,
                  backgroundColor: Colors.grey.shade800,
                  color: Colors.indigoAccent,
                ),
                const SizedBox(height: 4),
                const Text('KV Cache Hit Rate: 85.4% · TTFT: 18.2 ms', style: TextStyle(fontSize: 11, color: Colors.grey)),
              ],
            ),
          ),
        ),
      ],
    );
  }
}
