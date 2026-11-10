import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:umcf_flutter_client/src/ui/foundation_status_screen.dart';

void main() {
  group('[Integration] FoundationStatusScreen Responsive Layout & Actions', () {
    testWidgets('renders mobile layout (< 600px) with NavigationBar and binds interactive controls', (tester) async {
      // Set compact viewport: 390x844 (iPhone / Smartphone)
      tester.view.physicalSize = const Size(390, 844);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(
        const MaterialApp(
          home: FoundationStatusScreen(),
        ),
      );
      await tester.pumpAndSettle();

      // Verify mobile elements
      expect(find.byKey(const Key('mobile_nav_bar')), findsOneWidget);
      expect(find.byKey(const Key('desktop_nav_rail')), findsNothing);
      expect(find.text('UMCF Node Foundation (v0.1.0)'), findsOneWidget);
      expect(find.text('Node Status: Ready'), findsOneWidget);

      // Verify interactive controls (zero decorative buttons)
      final toggleFinder = find.byKey(const Key('sharing_toggle'));
      expect(toggleFinder, findsOneWidget);

      // Tap sharing toggle
      await tester.tap(toggleFinder);
      await tester.pumpAndSettle();

      // State transition should immediately reflect in the UI
      expect(find.text('Node Status: Active Worker'), findsOneWidget);

      // Tap refresh action button
      final refreshFinder = find.byKey(const Key('refresh_button'));
      expect(refreshFinder, findsOneWidget);
      await tester.tap(refreshFinder);
      await tester.pumpAndSettle();
    });

    testWidgets('renders desktop layout (>= 600px) with NavigationRail', (tester) async {
      // Set expanded viewport: 1440x900 (Desktop)
      tester.view.physicalSize = const Size(1440, 900);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(
        const MaterialApp(
          home: FoundationStatusScreen(),
        ),
      );
      await tester.pumpAndSettle();

      // Verify desktop elements
      expect(find.byKey(const Key('desktop_nav_rail')), findsOneWidget);
      expect(find.byKey(const Key('mobile_nav_bar')), findsNothing);
      expect(find.text('Node Status: Ready'), findsOneWidget);
    });
  });
}
