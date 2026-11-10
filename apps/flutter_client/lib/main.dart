import 'package:flutter/material.dart';

void main() {
  runApp(const UmcfFlutterClientApp());
}

class UmcfFlutterClientApp extends StatelessWidget {
  const UmcfFlutterClientApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'UMCF Flutter Client',
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark(useMaterial3: true),
      home: Scaffold(
        appBar: AppBar(
          title: const Text('UMCF Distributed Node'),
        ),
        body: const Center(
          child: Text('Universal Mobile Compute Fabric Client'),
        ),
      ),
    );
  }
}
