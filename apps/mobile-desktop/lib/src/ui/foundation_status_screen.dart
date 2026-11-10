import 'package:flutter/material.dart';
import '../models/hardware_capability.dart';

/// Foundation Status Screen displaying protocol version, node readiness, and hardware state.
/// Adapts responsively between mobile (< 600px) and desktop (>= 600px) viewports.
class FoundationStatusScreen extends StatefulWidget {
  final HardwareCapabilityDescriptor? initialDescriptor;

  const FoundationStatusScreen({super.key, this.initialDescriptor});

  @override
  State<FoundationStatusScreen> createState() => _FoundationStatusScreenState();
}

class _FoundationStatusScreenState extends State<FoundationStatusScreen> {
  int _selectedNavIndex = 0;
  bool _isSharingEnabled = false;
  String _nodeStatus = 'Ready';
  late DateTime _lastRefreshed;

  @override
  void initState() {
    super.initState();
    _lastRefreshed = DateTime.now();
  }

  void _refreshStatus() {
    setState(() {
      _lastRefreshed = DateTime.now();
      _nodeStatus = _isSharingEnabled ? 'Active Worker' : 'Ready (Sharing Disabled)';
    });
  }

  void _toggleSharing(bool value) {
    setState(() {
      _isSharingEnabled = value;
      _nodeStatus = value ? 'Active Worker' : 'Ready (Sharing Disabled)';
    });
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final isDesktop = constraints.maxWidth >= 600.0;

        return Scaffold(
          appBar: AppBar(
            title: const Text('UMCF Node Foundation (v0.1.0)'),
            actions: [
              IconButton(
                key: const Key('refresh_button'),
                icon: const Icon(Icons.refresh),
                tooltip: 'Refresh Status',
                onPressed: _refreshStatus,
              ),
            ],
          ),
          bottomNavigationBar: isDesktop
              ? null
              : NavigationBar(
                  key: const Key('mobile_nav_bar'),
                  selectedIndex: _selectedNavIndex,
                  onDestinationSelected: (index) {
                    setState(() => _selectedNavIndex = index);
                  },
                  destinations: const [
                    NavigationDestination(icon: Icon(Icons.dashboard), label: 'Overview'),
                    NavigationDestination(icon: Icon(Icons.memory), label: 'Hardware'),
                    NavigationDestination(icon: Icon(Icons.settings), label: 'Settings'),
                  ],
                ),
          body: Row(
            children: [
              if (isDesktop)
                NavigationRail(
                  key: const Key('desktop_nav_rail'),
                  selectedIndex: _selectedNavIndex,
                  onDestinationSelected: (index) {
                    setState(() => _selectedNavIndex = index);
                  },
                  labelType: NavigationRailLabelType.all,
                  destinations: const [
                    NavigationRailDestination(icon: Icon(Icons.dashboard), label: Text('Overview')),
                    NavigationRailDestination(icon: Icon(Icons.memory), label: Text('Hardware')),
                    NavigationRailDestination(icon: Icon(Icons.settings), label: Text('Settings')),
                  ],
                ),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.all(16.0),
                  child: ListView(
                    children: [
                      // Node Status Banner Card
                      Card(
                        elevation: 2,
                        child: Padding(
                          padding: const EdgeInsets.all(16.0),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Row(
                                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                                children: [
                                  Text(
                                    'Node Status: $_nodeStatus',
                                    style: Theme.of(context).textTheme.titleMedium?.copyWith(
                                          fontWeight: FontWeight.bold,
                                        ),
                                  ),
                                  Switch(
                                    key: const Key('sharing_toggle'),
                                    value: _isSharingEnabled,
                                    onChanged: _toggleSharing,
                                  ),
                                ],
                              ),
                              const SizedBox(height: 8),
                              Text(
                                'Protocol: UMCF Wire Envelope v0.1.0 (Magic 0x554D4346)',
                                style: Theme.of(context).textTheme.bodySmall,
                              ),
                              Text(
                                'Last Checked: ${_lastRefreshed.toIso8601String()}',
                                style: Theme.of(context).textTheme.bodySmall,
                              ),
                            ],
                          ),
                        ),
                      ),
                      const SizedBox(height: 16),

                      // Hardware Capabilities Overview Card
                      Card(
                        elevation: 2,
                        child: Padding(
                          padding: const EdgeInsets.all(16.0),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                'Hardware Capabilities',
                                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                                      fontWeight: FontWeight.bold,
                                    ),
                              ),
                              const Divider(),
                              ListTile(
                                leading: const Icon(Icons.computer),
                                title: const Text('CPU Architecture'),
                                subtitle: Text(widget.initialDescriptor?.cpu.architecture ?? 'Probed Host CPU (SIMD AVX2/NEON)'),
                              ),
                              ListTile(
                                leading: const Icon(Icons.memory),
                                title: const Text('Physical RAM'),
                                subtitle: Text(
                                  widget.initialDescriptor != null
                                      ? '${widget.initialDescriptor!.memory.totalRam ~/ (1024 * 1024 * 1024)} GiB Total'
                                      : 'Available System Memory',
                                ),
                              ),
                              ListTile(
                                leading: const Icon(Icons.battery_charging_full),
                                title: const Text('Power & Thermal State'),
                                subtitle: Text(
                                  widget.initialDescriptor != null
                                      ? 'Thermal: ${widget.initialDescriptor!.powerThermal.thermalState.toJson()}'
                                      : 'Thermal Status: Nominal',
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        );
      },
    );
  }
}
