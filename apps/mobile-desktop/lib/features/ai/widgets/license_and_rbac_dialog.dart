// Flutter dialog for model license agreement and RBAC visibility selection.
// Conforms to UMCF Rule 02: Zero non-functional UI controls.

import 'package:flutter/material.dart';

enum UiModelVisibility { privateLocalOnly, trustedClusterPeers, publicShared }

class LicenseAndRbacDialog extends StatefulWidget {
  final String modelName;
  final String modelId;
  final String licenseName;
  final String licenseSummary;
  final bool initialAccepted;
  final UiModelVisibility initialVisibility;
  final void Function(bool accepted, UiModelVisibility visibility) onSave;
  final VoidCallback onDismiss;

  const LicenseAndRbacDialog({
    super.key,
    required this.modelName,
    required this.modelId,
    required this.licenseName,
    required this.licenseSummary,
    required this.initialAccepted,
    required this.initialVisibility,
    required this.onSave,
    required this.onDismiss,
  });

  @override
  State<LicenseAndRbacDialog> createState() => _LicenseAndRbacDialogState();
}

class _LicenseAndRbacDialogState extends State<LicenseAndRbacDialog> {
  late bool _agreedToTerms;
  late UiModelVisibility _visibility;
  bool _isSigning = false;

  @override
  void initState() {
    super.initState();
    _agreedToTerms = widget.initialAccepted;
    _visibility = widget.initialVisibility;
  }

  void _handleConfirm() {
    setState(() {
      _isSigning = true;
    });
    // Record acceptance and persist visibility
    widget.onSave(_agreedToTerms, _visibility);
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      child: Padding(
        padding: const EdgeInsets.all(24.0),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  const Icon(Icons.verified_user, color: Colors.indigo, size: 28),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          widget.modelName,
                          style: const TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
                          overflow: TextOverflow.ellipsis,
                        ),
                        Text(
                          'License: ${widget.licenseName}',
                          style: TextStyle(fontSize: 12, color: Colors.indigo.shade700),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
              const Divider(height: 24),
              const Text('License Terms & Usage Agreement', style: TextStyle(fontWeight: FontWeight.bold)),
              const SizedBox(height: 8),
              Container(
                padding: const EdgeInsets.all(12),
                height: 120,
                decoration: BoxDecoration(
                  color: Colors.grey.shade50,
                  border: Border.all(color: Colors.grey.shade200),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: SingleChildScrollView(
                  child: Text(
                    widget.licenseSummary,
                    style: const TextStyle(fontSize: 11, height: 1.4),
                  ),
                ),
              ),
              const SizedBox(height: 12),
              CheckboxListTile(
                value: _agreedToTerms,
                dense: true,
                contentPadding: EdgeInsets.zero,
                activeColor: Colors.indigo,
                title: const Text(
                  'I accept the license terms on behalf of this node',
                  style: TextStyle(fontSize: 12, fontWeight: FontWeight.w600),
                ),
                onChanged: (val) {
                  setState(() {
                    _agreedToTerms = val ?? false;
                  });
                },
              ),
              const SizedBox(height: 16),
              const Text('Cluster Mesh Visibility (RBAC)', style: TextStyle(fontWeight: FontWeight.bold)),
              const SizedBox(height: 8),
              RadioListTile<UiModelVisibility>(
                value: UiModelVisibility.privateLocalOnly,
                groupValue: _visibility,
                dense: true,
                contentPadding: EdgeInsets.zero,
                activeColor: Colors.indigo,
                title: const Text('Private Local Only (Never shared or advertised)', style: TextStyle(fontSize: 12)),
                onChanged: (val) {
                  if (val != null) setState(() => _visibility = val);
                },
              ),
              RadioListTile<UiModelVisibility>(
                value: UiModelVisibility.trustedClusterPeers,
                groupValue: _visibility,
                dense: true,
                contentPadding: EdgeInsets.zero,
                activeColor: Colors.indigo,
                title: const Text('Trusted Cluster Peers (Verified enrollment required)', style: TextStyle(fontSize: 12)),
                onChanged: (val) {
                  if (val != null) setState(() => _visibility = val);
                },
              ),
              RadioListTile<UiModelVisibility>(
                value: UiModelVisibility.publicShared,
                groupValue: _visibility,
                dense: true,
                contentPadding: EdgeInsets.zero,
                activeColor: Colors.indigo,
                title: const Text('Public Shared (Full cluster P2P swarm distribution)', style: TextStyle(fontSize: 12)),
                onChanged: (val) {
                  if (val != null) setState(() => _visibility = val);
                },
              ),
              const SizedBox(height: 20),
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  TextButton(
                    onPressed: widget.onDismiss,
                    child: const Text('Cancel'),
                  ),
                  const SizedBox(width: 8),
                  ElevatedButton(
                    style: ElevatedButton.styleFrom(
                      backgroundColor: Colors.indigo,
                      foregroundColor: Colors.white,
                    ),
                    onPressed: _agreedToTerms && !_isSigning ? _handleConfirm : null,
                    child: _isSigning
                        ? const SizedBox(
                            width: 16,
                            height: 16,
                            child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                          )
                        : const Text('Sign & Save Policy'),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
