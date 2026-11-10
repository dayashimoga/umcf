import test from "node:test";
import assert from "node:assert/strict";
import { createClusterDashboardState, PROTOCOL_VERSION } from "../src/cluster-store.svelte.js";

test("[UNIT] dashboard initializes with safe non-sharing defaults and Hybrid role", () => {
  const state = createClusterDashboardState("cluster-01");
  assert.equal(PROTOCOL_VERSION, "1.0.0");
  assert.equal(state.clusterId, "cluster-01");
  assert.equal(state.localRole, "Hybrid");
  assert.equal(state.sharingEnabled, false);
  assert.equal(state.privateAiLocalOnly, true);
});

test("[UNIT] dashboard role transitions and consent validation enforce invariants", () => {
  const state = createClusterDashboardState();
  assert.equal(state.setRole("Master"), "Master");
  assert.equal(state.setRole("Worker"), "Worker");
  assert.throws(() => state.setRole("InvalidRole"), /Invalid node role/);

  state.updateConsent({ enabled: true, memoryMb: 4096, tempCelsius: 72 });
  assert.equal(state.sharingEnabled, true);
  assert.equal(state.maxMemoryMb, 4096);
  assert.equal(state.maxTemperatureCelsius, 72);

  assert.throws(() => state.updateConsent({ memoryMb: 16 }), /maxMemoryMb/);
  assert.throws(() => state.updateConsent({ tempCelsius: 120 }), /maxTemperatureCelsius/);
});

test("[UNIT] node registration filters out unverified hardware accelerators", () => {
  const state = createClusterDashboardState();
  const registered = state.registerNode({
    nodeId: "did:umcf:0123456789abcdef0123456789abcdef",
    role: "Worker",
    platform: "Windows",
    availableRamMb: 8192,
    accelerators: [
      { name: "Intel Arc 130T", backend: "Vulkan", verifiedOnHardware: true },
      { name: "Unprobed NPU", backend: "IntelNpu", verifiedOnHardware: false }
    ]
  });

  assert.equal(registered.verifiedAccelerators.length, 1);
  assert.equal(registered.verifiedAccelerators[0].name, "Intel Arc 130T");
  assert.equal(state.nodes.length, 1);
});

