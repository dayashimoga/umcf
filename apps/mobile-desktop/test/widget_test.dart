import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:umcf_mobile_desktop/main.dart';

void main() {
  testWidgets(
    '[UNIT] UMCF dashboard enforces safe consent defaults and functional controls',
    (WidgetTester tester) async {
      await tester.pumpWidget(const UmcfApp());

      expect(find.text('Active Role: HYBRID'), findsOneWidget);
      expect(
        find.text('Disabled by default until explicitly consented'),
        findsOneWidget,
      );

      // Cycle role: Hybrid -> Master -> Worker
      await tester.tap(find.byKey(const Key('btn_cycle_role')));
      await tester.pump();
      expect(find.text('Active Role: MASTER'), findsOneWidget);

      await tester.tap(find.byKey(const Key('btn_cycle_role')));
      await tester.pump();
      expect(find.text('Active Role: WORKER'), findsOneWidget);

      // Enable consent toggle
      await tester.tap(find.byKey(const Key('switch_consent_sharing')));
      await tester.pump();
      expect(find.text('Active (Max 2048 MiB RAM)'), findsOneWidget);

      // Adjust RAM limit
      await tester.tap(find.byKey(const Key('btn_increase_ram')));
      await tester.pump();
      expect(find.text('Active (Max 2560 MiB RAM)'), findsOneWidget);
    },
  );
}

