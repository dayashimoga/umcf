import 'package:flutter/material.dart';

void main() {
  runApp(const UmcfApp());
}

enum NodeRole { master, worker, hybrid }

class UmcfApp extends StatelessWidget {
  const UmcfApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Universal Mobile Compute Fabric',
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark(useMaterial3: true),
      home: const UmcfNodeDashboardScreen(),
    );
  }
}

class UmcfNodeDashboardScreen extends StatefulWidget {
  const UmcfNodeDashboardScreen({super.key});

  @override
  State<UmcfNodeDashboardScreen> createState() =>
      _UmcfNodeDashboardScreenState();
}

class _UmcfNodeDashboardScreenState extends State<UmcfNodeDashboardScreen> {
  NodeRole _role = NodeRole.hybrid;
  bool _sharingEnabled = false;
  bool _privateAiLocalOnly = true;
  bool _allowOnBattery = false;
  int _maxMemoryMb = 2048;

  void _cycleRole() {
    setState(() {
      switch (_role) {
        case NodeRole.hybrid:
          _role = NodeRole.master;
        case NodeRole.master:
          _role = NodeRole.worker;
        case NodeRole.worker:
          _role = NodeRole.hybrid;
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('UMCF Node Controller (v1.0.0)'),
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Card(
            child: ListTile(
              title: Text('Active Role: ${_role.name.toUpperCase()}'),
              subtitle: const Text(
                'Tap to dynamically transition between Master, Worker, and Hybrid',
              ),
              trailing: FilledButton(
                key: const Key('btn_cycle_role'),
                onPressed: _cycleRole,
                child: const Text('Switch Role'),
              ),
            ),
          ),
          SwitchListTile(
            key: const Key('switch_consent_sharing'),
            title: const Text('Resource Sharing Consent'),
            subtitle: Text(
              _sharingEnabled
                  ? 'Active (Max $_maxMemoryMb MiB RAM)'
                  : 'Disabled by default until explicitly consented',
            ),
            value: _sharingEnabled,
            onChanged: (bool value) {
              setState(() {
                _sharingEnabled = value;
              });
            },
          ),
          SwitchListTile(
            key: const Key('switch_private_ai'),
            title: const Text('Private AI Local-Only Mode'),
            subtitle: Text(
              _privateAiLocalOnly
                  ? 'Prompts and activations never leave this device'
                  : 'Distributed layer pipeline across enrolled cluster nodes',
            ),
            value: _privateAiLocalOnly,
            onChanged: (bool value) {
              setState(() {
                _privateAiLocalOnly = value;
              });
            },
          ),
          SwitchListTile(
            key: const Key('switch_battery_policy'),
            title: const Text('Allow Compute on Battery Power'),
            subtitle: Text(
              _allowOnBattery
                  ? 'Permitted above 50% battery charge'
                  : 'Charging required (Thermal & Battery Guard active)',
            ),
            value: _allowOnBattery,
            onChanged: (bool value) {
              setState(() {
                _allowOnBattery = value;
              });
            },
          ),
          Row(
            children: [
              Expanded(
                child: OutlinedButton(
                  key: const Key('btn_decrease_ram'),
                  onPressed: () {
                    setState(() {
                      if (_maxMemoryMb > 512) {
                        _maxMemoryMb -= 512;
                      }
                    });
                  },
                  child: const Text('-512 MiB RAM Limit'),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: OutlinedButton(
                  key: const Key('btn_increase_ram'),
                  onPressed: () {
                    setState(() {
                      _maxMemoryMb += 512;
                    });
                  },
                  child: const Text('+512 MiB RAM Limit'),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

