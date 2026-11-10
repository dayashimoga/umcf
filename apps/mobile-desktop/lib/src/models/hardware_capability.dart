// hardware_capability.dart — Typed Dart models matching schemas/hardware_capability_v1.schema.json
// Strictly adheres to zero-placeholder and production-grade validation invariants.

enum DeviceClass {
  smartphone,
  tablet,
  laptop,
  desktop,
  workstation,
  server,
  browserWasm;

  String toJson() {
    switch (this) {
      case DeviceClass.smartphone:
        return 'Smartphone';
      case DeviceClass.tablet:
        return 'Tablet';
      case DeviceClass.laptop:
        return 'Laptop';
      case DeviceClass.desktop:
        return 'Desktop';
      case DeviceClass.workstation:
        return 'Workstation';
      case DeviceClass.server:
        return 'Server';
      case DeviceClass.browserWasm:
        return 'BrowserWasm';
    }
  }

  static DeviceClass fromJson(String value) {
    switch (value) {
      case 'Smartphone':
        return DeviceClass.smartphone;
      case 'Tablet':
        return DeviceClass.tablet;
      case 'Laptop':
        return DeviceClass.laptop;
      case 'Desktop':
        return DeviceClass.desktop;
      case 'Workstation':
        return DeviceClass.workstation;
      case 'Server':
        return DeviceClass.server;
      case 'BrowserWasm':
        return DeviceClass.browserWasm;
      default:
        throw ArgumentError('Unknown DeviceClass: $value');
    }
  }
}

enum OperatingSystem {
  android,
  ios,
  windows,
  linux,
  macOs,
  web;

  String toJson() {
    switch (this) {
      case OperatingSystem.android:
        return 'Android';
      case OperatingSystem.ios:
        return 'Ios';
      case OperatingSystem.windows:
        return 'Windows';
      case OperatingSystem.linux:
        return 'Linux';
      case OperatingSystem.macOs:
        return 'MacOs';
      case OperatingSystem.web:
        return 'Web';
    }
  }

  static OperatingSystem fromJson(String value) {
    switch (value) {
      case 'Android':
        return OperatingSystem.android;
      case 'Ios':
        return OperatingSystem.ios;
      case 'Windows':
        return OperatingSystem.windows;
      case 'Linux':
        return OperatingSystem.linux;
      case 'MacOs':
        return OperatingSystem.macOs;
      case 'Web':
        return OperatingSystem.web;
      default:
        throw ArgumentError('Unknown OperatingSystem: $value');
    }
  }
}

enum AcceleratorKind {
  discreteGpu,
  integratedGpu,
  mobileGpu,
  npu,
  cpu;

  String toJson() {
    switch (this) {
      case AcceleratorKind.discreteGpu:
        return 'DiscreteGpu';
      case AcceleratorKind.integratedGpu:
        return 'IntegratedGpu';
      case AcceleratorKind.mobileGpu:
        return 'MobileGpu';
      case AcceleratorKind.npu:
        return 'Npu';
      case AcceleratorKind.cpu:
        return 'Cpu';
    }
  }

  static AcceleratorKind fromJson(String value) {
    switch (value) {
      case 'DiscreteGpu':
        return AcceleratorKind.discreteGpu;
      case 'IntegratedGpu':
        return AcceleratorKind.integratedGpu;
      case 'MobileGpu':
        return AcceleratorKind.mobileGpu;
      case 'Npu':
        return AcceleratorKind.npu;
      case 'Cpu':
        return AcceleratorKind.cpu;
      default:
        throw ArgumentError('Unknown AcceleratorKind: $value');
    }
  }
}

enum ComputeBackend {
  cpuSimd,
  vulkan,
  metal,
  dx12,
  cuda,
  rocm,
  openCl,
  nnapi,
  coreMl,
  webGpu;

  String toJson() {
    switch (this) {
      case ComputeBackend.cpuSimd:
        return 'CpuSimd';
      case ComputeBackend.vulkan:
        return 'Vulkan';
      case ComputeBackend.metal:
        return 'Metal';
      case ComputeBackend.dx12:
        return 'Dx12';
      case ComputeBackend.cuda:
        return 'Cuda';
      case ComputeBackend.rocm:
        return 'Rocm';
      case ComputeBackend.openCl:
        return 'OpenCl';
      case ComputeBackend.nnapi:
        return 'Nnapi';
      case ComputeBackend.coreMl:
        return 'CoreMl';
      case ComputeBackend.webGpu:
        return 'WebGpu';
    }
  }

  static ComputeBackend fromJson(String value) {
    switch (value) {
      case 'CpuSimd':
        return ComputeBackend.cpuSimd;
      case 'Vulkan':
        return ComputeBackend.vulkan;
      case 'Metal':
        return ComputeBackend.metal;
      case 'Dx12':
        return ComputeBackend.dx12;
      case 'Cuda':
        return ComputeBackend.cuda;
      case 'Rocm':
        return ComputeBackend.rocm;
      case 'OpenCl':
        return ComputeBackend.openCl;
      case 'Nnapi':
        return ComputeBackend.nnapi;
      case 'CoreMl':
        return ComputeBackend.coreMl;
      case 'WebGpu':
        return ComputeBackend.webGpu;
      default:
        throw ArgumentError('Unknown ComputeBackend: $value');
    }
  }
}

enum NetworkLinkType {
  unknown,
  loopback,
  ethernet,
  wifi,
  cellular5G,
  cellular4G;

  String toJson() {
    switch (this) {
      case NetworkLinkType.unknown:
        return 'Unknown';
      case NetworkLinkType.loopback:
        return 'Loopback';
      case NetworkLinkType.ethernet:
        return 'Ethernet';
      case NetworkLinkType.wifi:
        return 'Wifi';
      case NetworkLinkType.cellular5G:
        return 'Cellular5G';
      case NetworkLinkType.cellular4G:
        return 'Cellular4G';
    }
  }

  static NetworkLinkType fromJson(String value) {
    switch (value) {
      case 'Unknown':
        return NetworkLinkType.unknown;
      case 'Loopback':
        return NetworkLinkType.loopback;
      case 'Ethernet':
        return NetworkLinkType.ethernet;
      case 'Wifi':
        return NetworkLinkType.wifi;
      case 'Cellular5G':
        return NetworkLinkType.cellular5G;
      case 'Cellular4G':
        return NetworkLinkType.cellular4G;
      default:
        return NetworkLinkType.unknown;
    }
  }
}

enum ThermalState {
  nominal,
  fair,
  serious,
  critical;

  String toJson() {
    switch (this) {
      case ThermalState.nominal:
        return 'Nominal';
      case ThermalState.fair:
        return 'Fair';
      case ThermalState.serious:
        return 'Serious';
      case ThermalState.critical:
        return 'Critical';
    }
  }

  static ThermalState fromJson(String value) {
    switch (value) {
      case 'Nominal':
        return ThermalState.nominal;
      case 'Fair':
        return ThermalState.fair;
      case 'Serious':
        return ThermalState.serious;
      case 'Critical':
        return ThermalState.critical;
      default:
        throw ArgumentError('Unknown ThermalState: $value');
    }
  }
}

class CpuCapability {
  final String architecture;
  final int physicalCores;
  final int performanceCores;
  final int efficiencyCores;
  final bool simdAvx2;
  final bool simdAvx512;
  final bool simdNeon;
  final bool simdSve;
  final int baseClockMhz;
  final int boostClockMhz;

  const CpuCapability({
    required this.architecture,
    required this.physicalCores,
    required this.performanceCores,
    required this.efficiencyCores,
    required this.simdAvx2,
    required this.simdAvx512,
    required this.simdNeon,
    required this.simdSve,
    required this.baseClockMhz,
    required this.boostClockMhz,
  });

  factory CpuCapability.fromJson(Map<String, dynamic> json) {
    return CpuCapability(
      architecture: json['architecture'] as String,
      physicalCores: json['physical_cores'] as int,
      performanceCores: json['performance_cores'] as int,
      efficiencyCores: json['efficiency_cores'] as int,
      simdAvx2: json['simd_avx2'] as bool? ?? false,
      simdAvx512: json['simd_avx512'] as bool? ?? false,
      simdNeon: json['simd_neon'] as bool? ?? false,
      simdSve: json['simd_sve'] as bool? ?? false,
      baseClockMhz: json['base_clock_mhz'] as int,
      boostClockMhz: json['boost_clock_mhz'] as int,
    );
  }

  Map<String, dynamic> toJson() => {
    'architecture': architecture,
    'physical_cores': physicalCores,
    'performance_cores': performanceCores,
    'efficiency_cores': efficiencyCores,
    'simd_avx2': simdAvx2,
    'simd_avx512': simdAvx512,
    'simd_neon': simdNeon,
    'simd_sve': simdSve,
    'base_clock_mhz': baseClockMhz,
    'boost_clock_mhz': boostClockMhz,
  };
}

class MemoryCapability {
  final int totalRam;
  final int availableRam;
  final bool isUnifiedMemory;
  final int bandwidthMbps;

  const MemoryCapability({
    required this.totalRam,
    required this.availableRam,
    required this.isUnifiedMemory,
    required this.bandwidthMbps,
  });

  factory MemoryCapability.fromJson(Map<String, dynamic> json) {
    return MemoryCapability(
      totalRam: json['total_ram'] as int,
      availableRam: json['available_ram'] as int,
      isUnifiedMemory: json['is_unified_memory'] as bool? ?? false,
      bandwidthMbps: json['bandwidth_mbps'] as int,
    );
  }

  Map<String, dynamic> toJson() => {
    'total_ram': totalRam,
    'available_ram': availableRam,
    'is_unified_memory': isUnifiedMemory,
    'bandwidth_mbps': bandwidthMbps,
  };
}

class AcceleratorCapability {
  final String name;
  final AcceleratorKind kind;
  final ComputeBackend backend;
  final int vramBytes;
  final int dedicatedMemoryMb;
  final int sharedMemoryMb;
  final int computeUnits;
  final double fp32Tflops;
  final int fp16Tflops;
  final int int8Tops;
  final bool supportsFp16;
  final bool supportsInt8;
  final bool supportsInt4;
  final bool verifiedOnHardware;

  const AcceleratorCapability({
    required this.name,
    required this.kind,
    required this.backend,
    required this.vramBytes,
    required this.dedicatedMemoryMb,
    required this.sharedMemoryMb,
    required this.computeUnits,
    required this.fp32Tflops,
    required this.fp16Tflops,
    required this.int8Tops,
    required this.supportsFp16,
    required this.supportsInt8,
    required this.supportsInt4,
    required this.verifiedOnHardware,
  });

  factory AcceleratorCapability.fromJson(Map<String, dynamic> json) {
    return AcceleratorCapability(
      name: json['name'] as String,
      kind: AcceleratorKind.fromJson(json['kind'] as String),
      backend: ComputeBackend.fromJson(json['backend'] as String),
      vramBytes: json['vram_bytes'] as int,
      dedicatedMemoryMb: json['dedicated_memory_mb'] as int,
      sharedMemoryMb: json['shared_memory_mb'] as int,
      computeUnits: json['compute_units'] as int,
      fp32Tflops: (json['fp32_tflops'] as num).toDouble(),
      fp16Tflops: json['fp16_tflops'] as int,
      int8Tops: json['int8_tops'] as int,
      supportsFp16: json['supports_fp16'] as bool? ?? false,
      supportsInt8: json['supports_int8'] as bool? ?? false,
      supportsInt4: json['supports_int4'] as bool? ?? false,
      verifiedOnHardware: json['verified_on_hardware'] as bool? ?? false,
    );
  }

  Map<String, dynamic> toJson() => {
    'name': name,
    'kind': kind.toJson(),
    'backend': backend.toJson(),
    'vram_bytes': vramBytes,
    'dedicated_memory_mb': dedicatedMemoryMb,
    'shared_memory_mb': sharedMemoryMb,
    'compute_units': computeUnits,
    'fp32_tflops': fp32Tflops,
    'fp16_tflops': fp16Tflops,
    'int8_tops': int8Tops,
    'supports_fp16': supportsFp16,
    'supports_int8': supportsInt8,
    'supports_int4': supportsInt4,
    'verified_on_hardware': verifiedOnHardware,
  };
}

class StorageCapability {
  final int totalDisk;
  final int availableDisk;
  final bool isSolidState;

  const StorageCapability({
    required this.totalDisk,
    required this.availableDisk,
    required this.isSolidState,
  });

  factory StorageCapability.fromJson(Map<String, dynamic> json) {
    return StorageCapability(
      totalDisk: json['total_disk'] as int,
      availableDisk: json['available_disk'] as int,
      isSolidState: json['is_solid_state'] as bool? ?? true,
    );
  }

  Map<String, dynamic> toJson() => {
    'total_disk': totalDisk,
    'available_disk': availableDisk,
    'is_solid_state': isSolidState,
  };
}

class NetworkCapability {
  final NetworkLinkType linkType;
  final int estimatedBandwidthMbps;
  final int measuredRttMs;

  const NetworkCapability({
    required this.linkType,
    required this.estimatedBandwidthMbps,
    required this.measuredRttMs,
  });

  factory NetworkCapability.fromJson(Map<String, dynamic> json) {
    return NetworkCapability(
      linkType: NetworkLinkType.fromJson(json['link_type'] as String),
      estimatedBandwidthMbps: json['estimated_bandwidth_mbps'] as int,
      measuredRttMs: json['measured_rtt_ms'] as int,
    );
  }

  Map<String, dynamic> toJson() => {
    'link_type': linkType.toJson(),
    'estimated_bandwidth_mbps': estimatedBandwidthMbps,
    'measured_rtt_ms': measuredRttMs,
  };
}

class PowerThermalState {
  final int? batteryPercent;
  final bool isCharging;
  final ThermalState thermalState;
  final int? temperatureMilliCelsius;

  const PowerThermalState({
    this.batteryPercent,
    required this.isCharging,
    required this.thermalState,
    this.temperatureMilliCelsius,
  });

  factory PowerThermalState.fromJson(Map<String, dynamic> json) {
    return PowerThermalState(
      batteryPercent: json['battery_percent'] as int?,
      isCharging: json['is_charging'] as bool? ?? false,
      thermalState: ThermalState.fromJson(json['thermal_state'] as String),
      temperatureMilliCelsius: json['temperature_milli_celsius'] as int?,
    );
  }

  Map<String, dynamic> toJson() => {
    'battery_percent': batteryPercent,
    'is_charging': isCharging,
    'thermal_state': thermalState.toJson(),
    'temperature_milli_celsius': temperatureMilliCelsius,
  };
}

class HardwareCapabilityDescriptor {
  final String nodeId;
  final DeviceClass deviceClass;
  final OperatingSystem os;
  final CpuCapability cpu;
  final MemoryCapability memory;
  final List<AcceleratorCapability> accelerators;
  final StorageCapability storage;
  final NetworkCapability network;
  final PowerThermalState powerThermal;

  const HardwareCapabilityDescriptor({
    required this.nodeId,
    required this.deviceClass,
    required this.os,
    required this.cpu,
    required this.memory,
    required this.accelerators,
    required this.storage,
    required this.network,
    required this.powerThermal,
  });

  factory HardwareCapabilityDescriptor.fromJson(Map<String, dynamic> json) {
    return HardwareCapabilityDescriptor(
      nodeId: json['node_id'] as String,
      deviceClass: DeviceClass.fromJson(json['device_class'] as String),
      os: OperatingSystem.fromJson(json['os'] as String),
      cpu: CpuCapability.fromJson(json['cpu'] as Map<String, dynamic>),
      memory: MemoryCapability.fromJson(json['memory'] as Map<String, dynamic>),
      accelerators: (json['accelerators'] as List<dynamic>)
          .map((e) => AcceleratorCapability.fromJson(e as Map<String, dynamic>))
          .toList(),
      storage: StorageCapability.fromJson(json['storage'] as Map<String, dynamic>),
      network: NetworkCapability.fromJson(json['network'] as Map<String, dynamic>),
      powerThermal: PowerThermalState.fromJson(json['power_thermal'] as Map<String, dynamic>),
    );
  }

  Map<String, dynamic> toJson() => {
    'node_id': nodeId,
    'device_class': deviceClass.toJson(),
    'os': os.toJson(),
    'cpu': cpu.toJson(),
    'memory': memory.toJson(),
    'accelerators': accelerators.map((a) => a.toJson()).toList(),
    'storage': storage.toJson(),
    'network': network.toJson(),
    'power_thermal': powerThermal.toJson(),
  };
}
