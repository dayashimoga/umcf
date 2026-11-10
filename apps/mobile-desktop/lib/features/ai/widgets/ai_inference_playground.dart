// Flutter Interactive Inference Playground for Native llama.cpp & GGML Execution.
// Enforces UMCF Rule 02: Zero non-functional UI controls.

import 'dart:async';
import 'package:flutter/material.dart';

class BackendOption {
  final String id;
  final String name;
  final String description;
  final bool isGpu;

  const BackendOption({
    required this.id,
    required this.name,
    required this.description,
    required this.isGpu,
  });
}

class AiInferencePlayground extends StatefulWidget {
  final String modelName;
  final String modelId;
  final List<BackendOption> availableBackends;
  final Future<void> Function(String prompt, String backend, double temp, int topK, void Function(String chunk) onToken)? onGenerate;
  final VoidCallback? onAbort;

  const AiInferencePlayground({
    super.key,
    required this.modelName,
    required this.modelId,
    this.availableBackends = const [
      BackendOption(id: 'cpu', name: 'Host CPU (AVX2/NEON)', description: 'Simd multi-threaded execution', isGpu: false),
      BackendOption(id: 'vulkan', name: 'Vulkan GPU Acceleration', description: 'Cross-platform compute shader queue', isGpu: true),
      BackendOption(id: 'metal', name: 'Apple Silicon Metal', description: 'Unified memory GPU shaders', isGpu: true),
    ],
    this.onGenerate,
    this.onAbort,
  });

  @override
  State<AiInferencePlayground> createState() => _AiInferencePlaygroundState();
}

class _AiInferencePlaygroundState extends State<AiInferencePlayground> {
  final TextEditingController _promptController = TextEditingController(
    text: 'Explain how distributed llama.cpp pipeline parallelism works.',
  );
  late String _selectedBackend;
  double _temperature = 0.7;
  int _topK = 40;
  bool _useGreedy = false;

  bool _isGenerating = false;
  String _generatedOutput = '';
  int _tokenCount = 0;
  double _tokensPerSecond = 0.0;
  DateTime? _generationStart;

  @override
  void initState() {
    super.initState();
    _selectedBackend = widget.availableBackends.first.id;
  }

  @override
  void dispose() {
    _promptController.dispose();
    super.dispose();
  }

  void _handleStartGeneration() async {
    if (_promptController.text.trim().isEmpty) return;

    setState(() {
      _isGenerating = true;
      _generatedOutput = '';
      _tokenCount = 0;
      _tokensPerSecond = 0.0;
      _generationStart = DateTime.now();
    });

    final onTokenCallback = (String chunk) {
      if (!mounted) return;
      setState(() {
        _generatedOutput += chunk;
        _tokenCount++;
        final elapsed = DateTime.now().difference(_generationStart!).inMilliseconds / 1000.0;
        if (elapsed > 0) {
          _tokensPerSecond = _tokenCount / elapsed;
        }
      });
    };

    if (widget.onGenerate != null) {
      try {
        await widget.onGenerate!(
          _promptController.text,
          _selectedBackend,
          _useGreedy ? 0.0 : _temperature,
          _useGreedy ? 1 : _topK,
          onTokenCallback,
        );
      } catch (e) {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text('Generation error: $e'), backgroundColor: Colors.red),
          );
        }
      }
    } else {
      // Internal simulated streaming for standalone widget preview
      final mockTokens = [
        'Distributed', ' pipeline', ' parallelism', ' partitions', ' transformer',
        ' layers', ' sequentially', ' across', ' heterogeneous', ' devices', '.',
        ' Activations', ' flow', ' over', ' authenticated', ' QUIC', ' streams',
        ' while', ' intermediate', ' KV', ' caches', ' remain', ' local', '.'
      ];
      for (final t in mockTokens) {
        if (!_isGenerating) break;
        await Future.delayed(const Duration(milliseconds: 60));
        onTokenCallback(' $t');
      }
    }

    if (mounted) {
      setState(() {
        _isGenerating = false;
      });
    }
  }

  void _handleStopGeneration() {
    widget.onAbort?.call();
    setState(() {
      _isGenerating = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: Text('Inference Playground — ${widget.modelName}'),
        actions: [
          Padding(
            padding: const EdgeInsets.only(right: 16.0),
            child: Center(
              child: Text(
                _isGenerating ? '${_tokensPerSecond.toStringAsFixed(1)} T/s' : 'Idle',
                style: TextStyle(
                  color: _isGenerating ? Colors.greenAccent : Colors.grey,
                  fontWeight: FontWeight.bold,
                ),
              ),
            ),
          )
        ],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Backend Selector
            Card(
              elevation: 2,
              child: Padding(
                padding: const EdgeInsets.all(12.0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text('Hardware Backend', style: TextStyle(fontWeight: FontWeight.bold)),
                    const SizedBox(height: 8),
                    DropdownButtonFormField<String>(
                      value: _selectedBackend,
                      items: widget.availableBackends.map((b) {
                        return DropdownMenuItem(
                          value: b.id,
                          child: Text('${b.name} (${b.isGpu ? "GPU" : "CPU"})'),
                        );
                      }).toList(),
                      onChanged: _isGenerating ? null : (val) {
                        if (val != null) setState(() => _selectedBackend = val);
                      },
                      decoration: const InputDecoration(border: OutlineInputBorder()),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 12),

            // Sampler Controls
            Card(
              elevation: 2,
              child: Padding(
                padding: const EdgeInsets.all(12.0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        const Text('Sampler Configuration', style: TextStyle(fontWeight: FontWeight.bold)),
                        Row(
                          children: [
                            const Text('Greedy'),
                            Switch(
                              value: _useGreedy,
                              onChanged: _isGenerating ? null : (v) => setState(() => _useGreedy = v),
                            ),
                          ],
                        )
                      ],
                    ),
                    if (!_useGreedy) ...[
                      Text('Temperature: ${_temperature.toStringAsFixed(2)}'),
                      Slider(
                        value: _temperature,
                        min: 0.1,
                        max: 2.0,
                        divisions: 19,
                        onChanged: _isGenerating ? null : (v) => setState(() => _temperature = v),
                      ),
                      Text('Top-K: $_topK'),
                      Slider(
                        value: _topK.toDouble(),
                        min: 1,
                        max: 100,
                        divisions: 99,
                        onChanged: _isGenerating ? null : (v) => setState(() => _topK = v.toInt()),
                      ),
                    ],
                  ],
                ),
              ),
            ),
            const SizedBox(height: 12),

            // Prompt Input
            TextField(
              controller: _promptController,
              enabled: !_isGenerating,
              maxLines: 3,
              decoration: const InputDecoration(
                labelText: 'Prompt',
                border: OutlineInputBorder(),
                hintText: 'Enter inference prompt...',
              ),
            ),
            const SizedBox(height: 12),

            // Generate / Stop Action Buttons
            Row(
              children: [
                Expanded(
                  child: ElevatedButton.icon(
                    icon: Icon(_isGenerating ? Icons.stop : Icons.play_arrow),
                    label: Text(_isGenerating ? 'Stop Generation' : 'Generate Tokens'),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: _isGenerating ? Colors.red : theme.colorScheme.primary,
                      foregroundColor: Colors.white,
                      padding: const EdgeInsets.symmetric(vertical: 14),
                    ),
                    onPressed: _isGenerating ? _handleStopGeneration : _handleStartGeneration,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),

            // Real-time Token Output Stream Panel
            Card(
              color: Colors.black87,
              elevation: 3,
              child: Padding(
                padding: const EdgeInsets.all(16.0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        const Text('Generated Output', style: TextStyle(color: Colors.white70, fontWeight: FontWeight.bold)),
                        Text('Tokens: $_tokenCount', style: const TextStyle(color: Colors.white54, fontSize: 12)),
                      ],
                    ),
                    const Divider(color: Colors.white24),
                    const SizedBox(height: 8),
                    SelectableText(
                      _generatedOutput.isEmpty ? 'Waiting for generation...' : _generatedOutput,
                      style: const TextStyle(color: Colors.white, fontFamily: 'monospace', fontSize: 14),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
