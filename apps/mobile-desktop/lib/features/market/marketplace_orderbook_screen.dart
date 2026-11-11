import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Models an active compute ask offer in the marketplace.
class ComputeAskOfferModel {
  final String offerId;
  final String providerNodeId;
  final int cpuCores;
  final int ramMb;
  final String? gpuBackend;
  final int vramMb;
  final int npuTops;
  final bool onlyWhenCharging;
  final bool onlyOnUnmeteredWifi;
  final int maxThermalLevel;
  final bool isCharging;
  final bool isUnmeteredWifi;
  final int currentThermalLevel;
  final int baseFeeMicro;
  final int reputationScore;
  final bool isActive;

  const ComputeAskOfferModel({
    required this.offerId,
    required this.providerNodeId,
    required this.cpuCores,
    required this.ramMb,
    this.gpuBackend,
    required this.vramMb,
    required this.npuTops,
    required this.onlyWhenCharging,
    required this.onlyOnUnmeteredWifi,
    required this.maxThermalLevel,
    required this.isCharging,
    required this.isUnmeteredWifi,
    required this.currentThermalLevel,
    required this.baseFeeMicro,
    required this.reputationScore,
    required this.isActive,
  });
}

/// Models a workload bid request.
class ComputeBidRequestModel {
  final String bidId;
  final String requesterNodeId;
  final int minCpuCores;
  final int minRamMb;
  final bool requiresGpu;
  final int maxBudgetMicro;
  final String privacyTier;
  final int maxLatencyRttMs;

  const ComputeBidRequestModel({
    required this.bidId,
    required this.requesterNodeId,
    required this.minCpuCores,
    required this.minRamMb,
    required this.requiresGpu,
    required this.maxBudgetMicro,
    required this.privacyTier,
    required this.maxLatencyRttMs,
  });
}

/// Models an entry in the double-entry journal.
class JournalEntryModel {
  final String txId;
  final int timestampUnixMs;
  final String memo;
  final String debitAccount;
  final int debitAmountMicro;
  final String creditAccount;
  final int creditAmountMicro;

  const JournalEntryModel({
    required this.txId,
    required this.timestampUnixMs,
    required this.memo,
    required this.debitAccount,
    required this.debitAmountMicro,
    required this.creditAccount,
    required this.creditAmountMicro,
  });
}

/// State for the Marketplace & Ledger Screen.
class MarketplaceState {
  final int availableMicro;
  final int escrowLockedMicro;
  final int providerStakeMicro;
  final int slashedPoolMicro;
  final bool contributeActive;
  final bool onlyWhenCharging;
  final bool onlyOnWifi;
  final int maxThermalLevel;
  final bool isDeviceCharging;
  final bool isDeviceOnWifi;
  final List<ComputeAskOfferModel> asks;
  final List<ComputeBidRequestModel> bids;
  final List<JournalEntryModel> journal;

  const MarketplaceState({
    required this.availableMicro,
    required this.escrowLockedMicro,
    required this.providerStakeMicro,
    required this.slashedPoolMicro,
    required this.contributeActive,
    required this.onlyWhenCharging,
    required this.onlyOnWifi,
    required this.maxThermalLevel,
    required this.isDeviceCharging,
    required this.isDeviceOnWifi,
    required this.asks,
    required this.bids,
    required this.journal,
  });

  MarketplaceState copyWith({
    int? availableMicro,
    int? escrowLockedMicro,
    int? providerStakeMicro,
    int? slashedPoolMicro,
    bool? contributeActive,
    bool? onlyWhenCharging,
    bool? onlyOnWifi,
    int? maxThermalLevel,
    bool? isDeviceCharging,
    bool? isDeviceOnWifi,
    List<ComputeAskOfferModel>? asks,
    List<ComputeBidRequestModel>? bids,
    List<JournalEntryModel>? journal,
  }) {
    return MarketplaceState(
      availableMicro: availableMicro ?? this.availableMicro,
      escrowLockedMicro: escrowLockedMicro ?? this.escrowLockedMicro,
      providerStakeMicro: providerStakeMicro ?? this.providerStakeMicro,
      slashedPoolMicro: slashedPoolMicro ?? this.slashedPoolMicro,
      contributeActive: contributeActive ?? this.contributeActive,
      onlyWhenCharging: onlyWhenCharging ?? this.onlyWhenCharging,
      onlyOnWifi: onlyOnWifi ?? this.onlyOnWifi,
      maxThermalLevel: maxThermalLevel ?? this.maxThermalLevel,
      isDeviceCharging: isDeviceCharging ?? this.isDeviceCharging,
      isDeviceOnWifi: isDeviceOnWifi ?? this.isDeviceOnWifi,
      asks: asks ?? this.asks,
      bids: bids ?? this.bids,
      journal: journal ?? this.journal,
    );
  }
}

/// Controller managing marketplace and ledger state.
class MarketplaceNotifier extends StateNotifier<MarketplaceState> {
  MarketplaceNotifier()
      : super(const MarketplaceState(
          availableMicro: 85400000,
          escrowLockedMicro: 25000000,
          providerStakeMicro: 50000000,
          slashedPoolMicro: 0,
          contributeActive: true,
          onlyWhenCharging: true,
          onlyOnWifi: true,
          maxThermalLevel: 2,
          isDeviceCharging: true,
          isDeviceOnWifi: true,
          asks: [
            ComputeAskOfferModel(
              offerId: 'ask-desktop-4090',
              providerNodeId: 'did:umcf:desktop_rtx4090',
              cpuCores: 16,
              ramMb: 65536,
              gpuBackend: 'NVIDIA RTX 4090 (24 GB)',
              vramMb: 24576,
              npuTops: 0,
              onlyWhenCharging: false,
              onlyOnUnmeteredWifi: false,
              maxThermalLevel: 3,
              isCharging: true,
              isUnmeteredWifi: true,
              currentThermalLevel: 1,
              baseFeeMicro: 100,
              reputationScore: 985,
              isActive: true,
            ),
            ComputeAskOfferModel(
              offerId: 'ask-phone-pixel9',
              providerNodeId: 'did:umcf:phone_pixel9',
              cpuCores: 8,
              ramMb: 16384,
              gpuBackend: 'Mali-G715 (Vulkan)',
              vramMb: 8192,
              npuTops: 45,
              onlyWhenCharging: true,
              onlyOnUnmeteredWifi: true,
              maxThermalLevel: 2,
              isCharging: true,
              isUnmeteredWifi: true,
              currentThermalLevel: 0,
              baseFeeMicro: 50,
              reputationScore: 960,
              isActive: true,
            ),
          ],
          bids: [
            ComputeBidRequestModel(
              bidId: 'bid-ai-rag-batch',
              requesterNodeId: 'did:umcf:macbook_client',
              minCpuCores: 8,
              minRamMb: 16384,
              requiresGpu: true,
              maxBudgetMicro: 15000000,
              privacyTier: 'TrustedDevices',
              maxLatencyRttMs: 100,
            ),
          ],
          journal: [
            JournalEntryModel(
              txId: 'tx-001',
              timestampUnixMs: 1731322000000,
              memo: 'Initial developer token grant',
              debitAccount: 'treasury:mint',
              debitAmountMicro: 100000000,
              creditAccount: 'available:self',
              creditAmountMicro: 100000000,
            ),
            JournalEntryModel(
              txId: 'tx-002',
              timestampUnixMs: 1731322050000,
              memo: 'Deposit collateral stake for worker node',
              debitAccount: 'available:self',
              debitAmountMicro: 50000000,
              creditAccount: 'stake:self',
              creditAmountMicro: 50000000,
            ),
            JournalEntryModel(
              txId: 'tx-003',
              timestampUnixMs: 1731322100000,
              memo: 'Lock escrow budget for job-ai-rag-batch',
              debitAccount: 'available:self',
              debitAmountMicro: 15000000,
              creditAccount: 'escrow:job-ai-rag-batch',
              creditAmountMicro: 15000000,
            ),
          ],
        ));

  void toggleContribute() {
    state = state.copyWith(contributeActive: !state.contributeActive);
  }

  void toggleOnlyWhenCharging(bool value) {
    state = state.copyWith(onlyWhenCharging: value);
  }

  void toggleOnlyOnWifi(bool value) {
    state = state.copyWith(onlyOnWifi: value);
  }

  void toggleSimulatedCharging() {
    state = state.copyWith(isDeviceCharging: !state.isDeviceCharging);
  }

  void toggleSimulatedWifi() {
    state = state.copyWith(isDeviceOnWifi: !state.isDeviceOnWifi);
  }
}

final marketplaceProvider =
    StateNotifierProvider<MarketplaceNotifier, MarketplaceState>((ref) {
  return MarketplaceNotifier();
});

/// Material 3 Screen for Marketplace & Double-Entry Ledger.
class MarketplaceOrderbookScreen extends ConsumerWidget {
  const MarketplaceOrderbookScreen({super.key});

  String _formatCredits(int micro) {
    return (micro / 1000000).toStringAsFixed(6);
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(marketplaceProvider);
    final notifier = ref.read(marketplaceProvider.notifier);

    return DefaultTabController(
      length: 3,
      child: Scaffold(
        appBar: AppBar(
          title: const Text('Compute Marketplace & Ledger'),
          bottom: const TabBar(
            tabs: [
              Tab(icon: Icon(Icons.store), text: 'Order Book'),
              Tab(icon: Icon(Icons.safety_check), text: 'Device Safety'),
              Tab(icon: Icon(Icons.account_balance), text: 'Journal Ledger'),
            ],
          ),
        ),
        body: Column(
          children: [
            // Balance Overview Strip
            Container(
              padding: const EdgeInsets.all(16),
              color: Theme.of(context).colorScheme.surfaceContainerHighest,
              child: Row(
                children: [
                  Expanded(
                    child: _buildBalanceTile(
                      context,
                      'Available',
                      '${_formatCredits(state.availableMicro)} UMCF',
                      Colors.green,
                    ),
                  ),
                  Expanded(
                    child: _buildBalanceTile(
                      context,
                      'Escrow',
                      '${_formatCredits(state.escrowLockedMicro)} UMCF',
                      Colors.amber,
                    ),
                  ),
                  Expanded(
                    child: _buildBalanceTile(
                      context,
                      'Stake Bond',
                      '${_formatCredits(state.providerStakeMicro)} UMCF',
                      Colors.blue,
                    ),
                  ),
                ],
              ),
            ),
            // Tab Views
            Expanded(
              child: TabBarView(
                children: [
                  // Tab 1: Order Book Asks & Bids
                  ListView.builder(
                    padding: const EdgeInsets.all(16),
                    itemCount: state.asks.length,
                    itemBuilder: (context, index) {
                      final ask = state.asks[index];
                      final isEligible = ask.isActive &&
                          (!ask.onlyWhenCharging || ask.isCharging) &&
                          (!ask.onlyOnUnmeteredWifi || ask.isUnmeteredWifi);

                      return Card(
                        margin: const EdgeInsets.only(bottom: 12),
                        child: ListTile(
                          title: Text(ask.offerId,
                              style: const TextStyle(fontWeight: FontWeight.bold)),
                          subtitle: Text(
                              '${ask.cpuCores} cores · ${(ask.ramMb / 1024).round()} GB RAM · ${ask.gpuBackend ?? "CPU"}'),
                          trailing: Column(
                            mainAxisAlignment: MainAxisAlignment.center,
                            crossAxisAlignment: CrossAxisAlignment.end,
                            children: [
                              Text('${_formatCredits(ask.baseFeeMicro)} UMCF',
                                  style: const TextStyle(
                                      fontWeight: FontWeight.bold,
                                      color: Colors.green)),
                              Chip(
                                label: Text(isEligible ? 'Eligible' : 'Paused',
                                    style: const TextStyle(fontSize: 10)),
                                backgroundColor: isEligible
                                    ? Colors.green.withValues(alpha: 0.2)
                                    : Colors.orange.withValues(alpha: 0.2),
                                padding: EdgeInsets.zero,
                              ),
                            ],
                          ),
                        ),
                      );
                    },
                  ),
                  // Tab 2: Contribute Device Safety Settings
                  Padding(
                    padding: const EdgeInsets.all(16),
                    child: Card(
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            SwitchListTile(
                              title: const Text('Contribute My Device (Earn Credits)'),
                              subtitle: const Text(
                                  'Safely lend idle compute when mobile safety rules are met.'),
                              value: state.contributeActive,
                              onChanged: (_) => notifier.toggleContribute(),
                            ),
                            const Divider(),
                            CheckboxListTile(
                              title: const Text('Only when connected to AC Charger'),
                              subtitle: Row(
                                children: [
                                  Text(state.isDeviceCharging
                                      ? 'AC Connected'
                                      : 'On Battery'),
                                  TextButton(
                                    onPressed: notifier.toggleSimulatedCharging,
                                    child: const Text('[Simulate Plug/Unplug]'),
                                  ),
                                ],
                              ),
                              value: state.onlyWhenCharging,
                              onChanged: (v) => notifier.toggleOnlyWhenCharging(v ?? false),
                            ),
                            CheckboxListTile(
                              title: const Text('Only on Unmetered Wi-Fi / LAN'),
                              subtitle: Row(
                                children: [
                                  Text(state.isDeviceOnWifi
                                      ? 'Wi-Fi Active'
                                      : 'Cellular Data'),
                                  TextButton(
                                    onPressed: notifier.toggleSimulatedWifi,
                                    child: const Text('[Simulate Net Switch]'),
                                  ),
                                ],
                              ),
                              value: state.onlyOnWifi,
                              onChanged: (v) => notifier.toggleOnlyOnWifi(v ?? false),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                  // Tab 3: Double-Entry Journal Ledger
                  ListView.builder(
                    padding: const EdgeInsets.all(16),
                    itemCount: state.journal.length,
                    itemBuilder: (context, index) {
                      final entry = state.journal[index];
                      return Card(
                        margin: const EdgeInsets.only(bottom: 8),
                        child: ListTile(
                          title: Text(entry.memo,
                              style: const TextStyle(fontWeight: FontWeight.bold)),
                          subtitle: Text(
                              'Debit: ${entry.debitAccount} → Credit: ${entry.creditAccount}'),
                          trailing: Text(
                            '${_formatCredits(entry.debitAmountMicro)} UMCF',
                            style: const TextStyle(fontFamily: 'monospace'),
                          ),
                        ),
                      );
                    },
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildBalanceTile(
      BuildContext context, String title, String amount, Color color) {
    return Column(
      children: [
        Text(title, style: Theme.of(context).textTheme.labelSmall),
        const SizedBox(height: 4),
        Text(amount,
            style: TextStyle(
                fontWeight: FontWeight.bold, fontSize: 13, color: color)),
      ],
    );
  }
}

