import 'package:flutter/material.dart';

enum EnclavePrivacyMode {
  strictLocalOnly,
  trustedPersonalDevicesOnly,
  clusterEncrypted,
}

/// Private AI Enclave settings bottom sheet for data sovereignty and local privacy governance.
class PrivateAiEnclaveSettingsSheet extends StatefulWidget {
  final EnclavePrivacyMode initialMode;
  final bool initialZeroizeEnabled;
  final int outboundBytesSent;
  final ValueChanged<EnclavePrivacyMode>? onModeChanged;
  final ValueChanged<bool>? onZeroizeToggled;

  const PrivateAiEnclaveSettingsSheet({
    super.key,
    required this.initialMode,
    this.initialZeroizeEnabled = true,
    this.outboundBytesSent = 0,
    this.onModeChanged,
    this.onZeroizeToggled,
  });

  static Future<void> show(
    BuildContext context, {
    required EnclavePrivacyMode initialMode,
    bool initialZeroizeEnabled = true,
    int outboundBytesSent = 0,
    ValueChanged<EnclavePrivacyMode>? onModeChanged,
    ValueChanged<bool>? onZeroizeToggled,
  }) {
    return showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (ctx) => PrivateAiEnclaveSettingsSheet(
        initialMode: initialMode,
        initialZeroizeEnabled: initialZeroizeEnabled,
        outboundBytesSent: outboundBytesSent,
        onModeChanged: onModeChanged,
        onZeroizeToggled: onZeroizeToggled,
      ),
    );
  }

  @override
  State<PrivateAiEnclaveSettingsSheet> createState() => _PrivateAiEnclaveSettingsSheetState();
}

class _PrivateAiEnclaveSettingsSheetState extends State<PrivateAiEnclaveSettingsSheet> {
  late EnclavePrivacyMode _selectedMode;
  late bool _zeroizeEnabled;

  @override
  void initState() {
    super.initState();
    _selectedMode = widget.initialMode;
    _zeroizeEnabled = widget.initialZeroizeEnabled;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(20, 16, 20, 24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Drag handle
            Center(
              child: Container(
                width: 40,
                height: 4,
                margin: const EdgeInsets.only(bottom: 16),
                decoration: BoxDecoration(
                  color: theme.colorScheme.outlineVariant,
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
            ),

            // Title & Live Proof Shield
            Row(
              children: [
                Icon(
                  _selectedMode == EnclavePrivacyMode.strictLocalOnly
                      ? Icons.verified_user
                      : Icons.security,
                  color: _selectedMode == EnclavePrivacyMode.strictLocalOnly
                      ? Colors.green
                      : theme.colorScheme.primary,
                  size: 28,
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Private AI Privacy Enclave',
                        style: theme.textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
                      ),
                      Text(
                        'Hardware-Enforced Data Sovereignty',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
                // Live Outbound Egress Counter
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                  decoration: BoxDecoration(
                    color: _selectedMode == EnclavePrivacyMode.strictLocalOnly && widget.outboundBytesSent == 0
                        ? Colors.green.withOpacity(0.15)
                        : Colors.orange.withOpacity(0.15),
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(
                      color: _selectedMode == EnclavePrivacyMode.strictLocalOnly && widget.outboundBytesSent == 0
                          ? Colors.green.withOpacity(0.4)
                          : Colors.orange.withOpacity(0.4),
                    ),
                  ),
                  child: Text(
                    'Outbound Egress: ${widget.outboundBytesSent} B',
                    style: TextStyle(
                      fontSize: 11,
                      fontWeight: FontWeight.bold,
                      color: _selectedMode == EnclavePrivacyMode.strictLocalOnly && widget.outboundBytesSent == 0
                          ? Colors.green.shade800
                          : Colors.orange.shade800,
                      fontFamily: 'monospace',
                    ),
                  ),
                ),
              ],
            ),
            const Divider(height: 24),

            // Mode Selector
            Text('Enclave Isolation Level', style: theme.textTheme.labelLarge),
            const SizedBox(height: 8),

            RadioListTile<EnclavePrivacyMode>(
              value: EnclavePrivacyMode.strictLocalOnly,
              groupValue: _selectedMode,
              onChanged: (mode) {
                if (mode != null) {
                  setState(() => _selectedMode = mode);
                  widget.onModeChanged?.call(mode);
                }
              },
              title: const Text('Strict Local-Only (Recommended)', style: TextStyle(fontWeight: FontWeight.bold)),
              subtitle: const Text('Zero outbound network packets. Inference executes strictly on this local device.'),
              activeColor: Colors.green,
              secondary: const Icon(Icons.lock, color: Colors.green),
            ),

            RadioListTile<EnclavePrivacyMode>(
              value: EnclavePrivacyMode.trustedPersonalDevicesOnly,
              groupValue: _selectedMode,
              onChanged: (mode) {
                if (mode != null) {
                  setState(() => _selectedMode = mode);
                  widget.onModeChanged?.call(mode);
                }
              },
              title: const Text('Trusted Personal Devices Only'),
              subtitle: const Text('Allow sharding across devices signed with your primary cryptographic identity key.'),
              activeColor: theme.colorScheme.primary,
              secondary: const Icon(Icons.devices, color: Colors.blue),
            ),

            RadioListTile<EnclavePrivacyMode>(
              value: EnclavePrivacyMode.clusterEncrypted,
              groupValue: _selectedMode,
              onChanged: (mode) {
                if (mode != null) {
                  setState(() => _selectedMode = mode);
                  widget.onModeChanged?.call(mode);
                }
              },
              title: const Text('Cluster Encrypted (Mesh)'),
              subtitle: const Text('Activations encrypted in-transit via ChaCha20-Poly1305 with ephemeral session keys.'),
              activeColor: Colors.amber.shade800,
              secondary: const Icon(Icons.vpn_lock, color: Colors.amber),
            ),

            const SizedBox(height: 12),
            // Zeroize RAM Switch
            SwitchListTile(
              value: _zeroizeEnabled,
              onChanged: (val) {
                setState(() => _zeroizeEnabled = val);
                widget.onZeroizeToggled?.call(val);
              },
              title: const Text('Zeroize Physical RAM Pages', style: TextStyle(fontWeight: FontWeight.bold)),
              subtitle: const Text('Overwrites RAM holding prompt and intermediate activations with 0x00 upon completion.'),
              secondary: const Icon(Icons.memory),
            ),

            const SizedBox(height: 16),
            // Close Button
            SizedBox(
              width: double.infinity,
              child: FilledButton(
                onPressed: () => Navigator.of(context).pop(),
                child: const Text('Apply Enclave Policy'),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

