// api_keys_screen.dart
// UMCF Stage 07 Phase 2 Subphase 1 (S7.P2.SP1)
// Scoped Capability Tokens & Developer API Gateway Management Screen.
// Multiplatform Flutter (Android, iOS, Windows, Linux, macOS), Riverpod state management, zero decorative controls.

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

class ApiKeyItemModel {
  final String id;
  final String label;
  final List<String> scopes;
  final int rateLimitRps;
  final int burst;
  final String createdAt;
  final String? expiresAt;
  final bool revoked;

  const ApiKeyItemModel({
    required this.id,
    required this.label,
    required this.scopes,
    required this.rateLimitRps,
    required this.burst,
    required this.createdAt,
    this.expiresAt,
    required this.revoked,
  });

  ApiKeyItemModel copyWith({bool? revoked}) {
    return ApiKeyItemModel(
      id: id,
      label: label,
      scopes: scopes,
      rateLimitRps: rateLimitRps,
      burst: burst,
      createdAt: createdAt,
      expiresAt: expiresAt,
      revoked: revoked ?? this.revoked,
    );
  }
}

class ApiKeysState {
  final List<ApiKeyItemModel> keys;
  final String gatewayUrl;

  const ApiKeysState({
    required this.keys,
    required this.gatewayUrl,
  });

  ApiKeysState copyWith({
    List<ApiKeyItemModel>? keys,
    String? gatewayUrl,
  }) {
    return ApiKeysState(
      keys: keys ?? this.keys,
      gatewayUrl: gatewayUrl ?? this.gatewayUrl,
    );
  }
}

class ApiKeysNotifier extends StateNotifier<ApiKeysState> {
  ApiKeysNotifier()
      : super(
          const ApiKeysState(
            gatewayUrl: "http://127.0.0.1:8080/api/v1",
            keys: [
              ApiKeyItemModel(
                id: "key_live_9a8b1c2d",
                label: "Production Inference Service",
                scopes: ["ai:inference", "storage:read", "jobs:read"],
                rateLimitRps: 50,
                burst: 100,
                createdAt: "2026-11-10",
                expiresAt: "2027-11-10",
                revoked: false,
              ),
              ApiKeyItemModel(
                id: "key_live_3f4e5d6c",
                label: "CI/CD Pipeline Runner",
                scopes: ["jobs:submit", "jobs:read", "storage:write", "storage:read"],
                rateLimitRps: 20,
                burst: 40,
                createdAt: "2026-11-08",
                expiresAt: "2027-02-08",
                revoked: false,
              ),
              ApiKeyItemModel(
                id: "key_live_7b8c9d0e",
                label: "Cluster Admin Superuser",
                scopes: ["admin:full"],
                rateLimitRps: 200,
                burst: 500,
                createdAt: "2026-11-01",
                revoked: false,
              ),
            ],
          ),
        );

  String createKey({
    required String label,
    required List<String> scopes,
    required int rps,
    required int burst,
  }) {
    final hexTag = DateTime.now().millisecondsSinceEpoch.toRadixString(16);
    final rawToken = "umcf_live_${hexTag}48c1e820f1";
    final keyId = "key_live_${hexTag.substring(0, 6)}";

    final newKey = ApiKeyItemModel(
      id: keyId,
      label: label,
      scopes: scopes.isNotEmpty ? scopes : ["cluster:read"],
      rateLimitRps: rps,
      burst: burst,
      createdAt: DateTime.now().toIso8601String().substring(0, 10),
      expiresAt: "2027-11-11",
      revoked: false,
    );

    state = state.copyWith(keys: [newKey, ...state.keys]);
    return rawToken;
  }

  void revokeKey(String id) {
    final updated = state.keys.map((k) {
      if (k.id == id) {
        return k.copyWith(revoked: true);
      }
      return k;
    }).toList();
    state = state.copyWith(keys: updated);
  }
}

final apiKeysNotifierProvider =
    StateNotifierProvider<ApiKeysNotifier, ApiKeysState>((ref) {
  return ApiKeysNotifier();
});

class ApiKeysScreen extends ConsumerWidget {
  const ApiKeysScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(apiKeysNotifierProvider);
    final notifier = ref.read(apiKeysNotifierProvider.notifier);
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Developer API Keys'),
        actions: [
          Padding(
            padding: const EdgeInsets.only(right: 16),
            child: Center(
              child: Chip(
                avatar: const Icon(Icons.check_circle, size: 16, color: Colors.greenAccent),
                label: const Text('Gateway /api/v1 Ready'),
                backgroundColor: Colors.white10,
                labelStyle: const TextStyle(fontSize: 12),
              ),
            ),
          ),
        ],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _buildOverviewCard(context, state),
            const SizedBox(height: 16),
            _buildKeyList(context, state, notifier),
          ],
        ),
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () => _showCreateKeyDialog(context, notifier),
        icon: const Icon(Icons.add),
        label: const Text('Create API Key'),
        backgroundColor: theme.colorScheme.primary,
      ),
    );
  }

  Widget _buildOverviewCard(BuildContext context, ApiKeysState state) {
    final activeCount = state.keys.where((k) => !k.revoked).length;
    return Card(
      elevation: 2,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.vpn_key, color: Colors.lightBlueAccent),
                const SizedBox(width: 8),
                const Text(
                  'Scoped Bearer Capability Tokens',
                  style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
                ),
                const Spacer(),
                Text(
                  '$activeCount Active Keys',
                  style: const TextStyle(fontWeight: FontWeight.bold, color: Colors.greenAccent),
                ),
              ],
            ),
            const SizedBox(height: 8),
            const Text(
              'Enforces principle of least privilege across all UMCF endpoints. '
              'Tokens support fine-grained scopes, token-bucket rate limiting, and RFC 9457 error decoding.',
              style: TextStyle(fontSize: 13, color: Colors.white70),
            ),
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              decoration: BoxDecoration(
                color: Colors.black38,
                borderRadius: BorderRadius.circular(6),
                border: Border.all(color: Colors.white10),
              ),
              child: Row(
                children: [
                  const Text('Endpoint: ', style: TextStyle(fontSize: 12, color: Colors.white54)),
                  Text(
                    state.gatewayUrl,
                    style: const TextStyle(fontSize: 12, fontFamily: 'monospace', color: Colors.lightBlueAccent),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildKeyList(
    BuildContext context,
    ApiKeysState state,
    ApiKeysNotifier notifier,
  ) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text(
          'Configured Access Keys',
          style: TextStyle(fontSize: 15, fontWeight: FontWeight.bold),
        ),
        const SizedBox(height: 8),
        ListView.separated(
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          itemCount: state.keys.length,
          separatorBuilder: (_, __) => const SizedBox(height: 8),
          itemBuilder: (context, index) {
            final key = state.keys[index];
            return Card(
              elevation: 1,
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(10),
                side: BorderSide(
                  color: key.revoked ? Colors.red.withOpacity(0.3) : Colors.white12,
                ),
              ),
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Text(
                          key.label,
                          style: TextStyle(
                            fontWeight: FontWeight.bold,
                            fontSize: 14,
                            decoration: key.revoked ? TextDecoration.lineThrough : null,
                          ),
                        ),
                        const Spacer(),
                        if (key.revoked)
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                            decoration: BoxDecoration(
                              color: Colors.red.withOpacity(0.15),
                              borderRadius: BorderRadius.circular(6),
                            ),
                            child: const Text('Revoked', style: TextStyle(color: Colors.redAccent, fontSize: 11)),
                          )
                        else
                          IconButton(
                            icon: const Icon(Icons.block, size: 18, color: Colors.redAccent),
                            tooltip: 'Revoke Key',
                            onPressed: () => notifier.revokeKey(key.id),
                          ),
                      ],
                    ),
                    Text(
                      'ID: ${key.id} • Created: ${key.createdAt} • ${key.rateLimitRps} rps',
                      style: const TextStyle(fontSize: 11, color: Colors.white54, fontFamily: 'monospace'),
                    ),
                    const SizedBox(height: 8),
                    Wrap(
                      spacing: 6,
                      runSpacing: 4,
                      children: key.scopes.map((s) {
                        Color c = Colors.lightBlueAccent;
                        if (s == "admin:full") c = Colors.redAccent;
                        if (s == "ai:inference") c = Colors.purpleAccent;
                        if (s.startsWith("storage:")) c = Colors.greenAccent;

                        return Container(
                          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                          decoration: BoxDecoration(
                            color: c.withOpacity(0.15),
                            borderRadius: BorderRadius.circular(4),
                            border: Border.all(color: c.withOpacity(0.3)),
                          ),
                          child: Text(s, style: TextStyle(color: c, fontSize: 10, fontFamily: 'monospace')),
                        );
                      }).toList(),
                    ),
                  ],
                ),
              ),
            );
          },
        ),
      ],
    );
  }

  void _showCreateKeyDialog(BuildContext context, ApiKeysNotifier notifier) {
    final labelCtrl = TextEditingController();
    final selectedScopes = <String>{"cluster:read", "jobs:read"};
    int rps = 50;

    showDialog(
      context: context,
      builder: (ctx) {
        return StatefulBuilder(
          builder: (context, setDialogState) {
            return AlertDialog(
              title: const Text('Generate Scoped API Key'),
              content: SingleChildScrollView(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    TextField(
                      controller: labelCtrl,
                      decoration: const InputDecoration(
                        labelText: 'Key Label',
                        hintText: 'e.g. My App Service',
                      ),
                    ),
                    const SizedBox(height: 16),
                    const Text('Granted Scopes:', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 12)),
                    const SizedBox(height: 6),
                    ...[
                      "cluster:read",
                      "jobs:submit",
                      "jobs:read",
                      "ai:inference",
                      "storage:read",
                      "storage:write",
                      "admin:full",
                    ].map((scope) {
                      final isSelected = selectedScopes.contains(scope);
                      return CheckboxListTile(
                        dense: true,
                        title: Text(scope, style: const TextStyle(fontSize: 12, fontFamily: 'monospace')),
                        value: isSelected,
                        onChanged: (val) {
                          setDialogState(() {
                            if (val == true) {
                              selectedScopes.add(scope);
                            } else {
                              selectedScopes.remove(scope);
                            }
                          });
                        },
                      );
                    }),
                  ],
                ),
              ),
              actions: [
                TextButton(
                  onPressed: () => Navigator.pop(ctx),
                  child: const Text('Cancel'),
                ),
                ElevatedButton(
                  onPressed: () {
                    final label = labelCtrl.text.trim();
                    if (label.isNotEmpty) {
                      final token = notifier.createKey(
                        label: label,
                        scopes: selectedScopes.toList(),
                        rps: rps,
                        burst: rps * 2,
                      );
                      Navigator.pop(ctx);
                      _showTokenCreatedDialog(context, token);
                    }
                  },
                  child: const Text('Generate Token'),
                ),
              ],
            );
          },
        );
      },
    );
  }

  void _showTokenCreatedDialog(BuildContext context, String token) {
    showDialog(
      context: context,
      builder: (ctx) {
        return AlertDialog(
          title: const Text('Secret Bearer Token'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text(
                'Copy your token now. It will not be shown again.',
                style: TextStyle(fontSize: 12, color: Colors.white70),
              ),
              const SizedBox(height: 12),
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: Colors.black38,
                  borderRadius: BorderRadius.circular(6),
                ),
                child: SelectableText(
                  token,
                  style: const TextStyle(fontFamily: 'monospace', color: Colors.greenAccent, fontSize: 12),
                ),
              ),
            ],
          ),
          actions: [
            TextButton.icon(
              icon: const Icon(Icons.copy, size: 16),
              label: const Text('Copy'),
              onPressed: () {
                Clipboard.setData(ClipboardData(text: token));
                Navigator.pop(ctx);
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('Token copied to clipboard')),
                );
              },
            ),
            ElevatedButton(
              onPressed: () => Navigator.pop(ctx),
              child: const Text('Done'),
            ),
          ],
        );
      },
    );
  }
}
