import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:umcf_mobile_desktop/src/ui/foundation_status_screen.dart';

void main() {
  group('[Integration] FoundationStatusScreen Responsive Layout & Actions', () {
    testWidgets('renders mobile layout (< 600px) with NavigationBar and binds interactive controls', (tester) async {
      tester.view.physicalSize = const Size(390, 844);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(
        const MaterialApp(
          home: FoundationStatusScreen(),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('mobile_nav_bar')), findsOneWidget);
      expect(find.byKey(const Key('desktop_nav_rail')), findsNothing);
      expect(find.text('UMCF Node Foundation (v0.1.0)'), findsOneWidget);
      expect(find.text('Node Status: Ready'), findsOneWidget);

      final toggleFinder = find.byKey(const Key('sharing_toggle'));
      expect(toggleFinder, findsOneWidget);

      await tester.tap(toggleFinder);
      await tester.pumpAndSettle();

      expect(find.text('Node Status: Active Worker'), findsOneWidget);

      final refreshFinder = find.byKey(const Key('refresh_button'));
      expect(refreshFinder, findsOneWidget);
      await tester.tap(refreshFinder);
      await tester.pumpAndSettle();
    });

    testWidgets('renders desktop layout (>= 600px) with NavigationRail', (tester) async {
      tester.view.physicalSize = const Size(1440, 900);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(
        const MaterialApp(
          home: FoundationStatusScreen(),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('desktop_nav_rail')), findsOneWidget);
      expect(find.byKey(const Key('mobile_nav_bar')), findsNothing);
      expect(find.text('Node Status: Ready'), findsOneWidget);
    });
  });
}
