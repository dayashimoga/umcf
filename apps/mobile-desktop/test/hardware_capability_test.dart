import 'package:flutter_test/flutter_test.dart';
import 'package:umcf_mobile_desktop/src/models/hardware_capability.dart';

void main() {
  group('[Unit] HardwareCapabilityDescriptor Model & JSON Parity', () {
    test('deserializes golden Rust fixture and performs roundtrip serialization', () {
      final sampleJson = {
        'node_id': 'did:umcf:a1b2c3d4e5f60718293a4b5c6d7e8f90',
        'device_class': 'Laptop',
        'os': 'Linux',
        'cpu': {
          'architecture': 'x86_64',
          'physical_cores': 8,
          'performance_cores': 4,
          'efficiency_cores': 4,
          'simd_avx2': true,
          'simd_avx512': false,
          'simd_neon': false,
          'simd_sve': false,
          'base_clock_mhz': 2400,
          'boost_clock_mhz': 4800,
        },
        'memory': {
          'total_ram': 17179869184,
          'available_ram': 12884901888,
          'is_unified_memory': false,
          'bandwidth_mbps': 51200,
        },
        'accelerators': [
          {
            'name': 'NVIDIA RTX 4070 Laptop GPU',
            'kind': 'DiscreteGpu',
            'backend': 'Cuda',
            'vram_bytes': 8589934592,
            'dedicated_memory_mb': 8192,
            'shared_memory_mb': 0,
            'compute_units': 36,
            'fp32_tflops': 16.5,
            'fp16_tflops': 33,
            'int8_tops': 66,
            'supports_fp16': true,
            'supports_int8': true,
            'supports_int4': true,
            'verified_on_hardware': true,
          }
        ],
        'storage': {
          'total_disk': 1000204886016,
          'available_disk': 500102443008,
          'is_solid_state': true,
        },
        'network': {
          'link_type': 'Wifi',
          'estimated_bandwidth_mbps': 600,
          'measured_rtt_ms': 12,
        },
        'power_thermal': {
          'battery_percent': 85,
          'is_charging': true,
          'thermal_state': 'Nominal',
          'temperature_milli_celsius': 42000,
        },
      };

      // 1. Parse JSON
      final descriptor = HardwareCapabilityDescriptor.fromJson(sampleJson);
      expect(descriptor.nodeId, 'did:umcf:a1b2c3d4e5f60718293a4b5c6d7e8f90');
      expect(descriptor.deviceClass, DeviceClass.laptop);
      expect(descriptor.os, OperatingSystem.linux);
      expect(descriptor.cpu.physicalCores, 8);
      expect(descriptor.cpu.simdAvx2, isTrue);
      expect(descriptor.accelerators.length, 1);
      expect(descriptor.accelerators.first.backend, ComputeBackend.cuda);
      expect(descriptor.accelerators.first.fp32Tflops, 16.5);
      expect(descriptor.powerThermal.thermalState, ThermalState.nominal);
      expect(descriptor.powerThermal.isCharging, isTrue);

      // 2. Round-trip serialization
      final serialized = descriptor.toJson();
      final roundtripDescriptor = HardwareCapabilityDescriptor.fromJson(serialized);

      expect(roundtripDescriptor.nodeId, descriptor.nodeId);
      expect(roundtripDescriptor.deviceClass, descriptor.deviceClass);
      expect(roundtripDescriptor.os, descriptor.os);
      expect(roundtripDescriptor.memory.totalRam, descriptor.memory.totalRam);
      expect(roundtripDescriptor.accelerators.first.name, descriptor.accelerators.first.name);
    });

    test('[Simulation] rejects malformed JSON with typed error', () {
      final malformedJson = {
        'node_id': 'did:umcf:invalid',
        'device_class': 'NonExistentDevice',
      };

      expect(
        () => HardwareCapabilityDescriptor.fromJson(malformedJson),
        throwsA(isA<ArgumentError>()),
      );
    });
  });
}
