// UMCF Svelte 5 Cluster State & Capability Negotiation Controller
// Enforces Control-Plane vs. Data-Plane separation and explicit hardware verification state.

export const PROTOCOL_VERSION = "1.0.0";
export const MAX_CONTROL_PLANE_BYTES = 65536;

export function createClusterDashboardState(initialClusterId = "umcf-lan-mesh") {
  let clusterId = initialClusterId;
  let localRole = "Hybrid";
  let sharingEnabled = false;
  let maxMemoryMb = 2048;
  let maxTemperatureCelsius = 75;
  let privateAiLocalOnly = true;
  let nodes = [];

  return {
    get clusterId() {
      return clusterId;
    },
    get localRole() {
      return localRole;
    },
    get sharingEnabled() {
      return sharingEnabled;
    },
    get maxMemoryMb() {
      return maxMemoryMb;
    },
    get maxTemperatureCelsius() {
      return maxTemperatureCelsius;
    },
    get privateAiLocalOnly() {
      return privateAiLocalOnly;
    },
    get nodes() {
      return [...nodes];
    },
    setRole(newRole) {
      if (!["Master", "Worker", "Hybrid"].includes(newRole)) {
        throw new Error(`Invalid node role: ${newRole}`);
      }
      localRole = newRole;
      return localRole;
    },
    updateConsent({ enabled, memoryMb, tempCelsius }) {
      if (typeof memoryMb === "number" && memoryMb < 64) {
        throw new Error("maxMemoryMb must be >= 64 MiB");
      }
      if (typeof tempCelsius === "number" && (tempCelsius < 35 || tempCelsius > 105)) {
        throw new Error("maxTemperatureCelsius must be between 35 and 105 C");
      }
      if (typeof enabled === "boolean") sharingEnabled = enabled;
      if (typeof memoryMb === "number") maxMemoryMb = memoryMb;
      if (typeof tempCelsius === "number") maxTemperatureCelsius = tempCelsius;
    },
    setPrivateAiMode(localOnly) {
      privateAiLocalOnly = Boolean(localOnly);
    },
    registerNode(node) {
      if (!node || typeof node.nodeId !== "string" || !node.nodeId.startsWith("did:umcf:")) {
        throw new Error("Invalid NodeId format");
      }
      const verifiedAccelerators = (node.accelerators || []).filter(
        (acc) => acc.verifiedOnHardware === true
      );
      const entry = {
        nodeId: node.nodeId,
        role: node.role || "Worker",
        platform: node.platform || "Linux",
        availableRamMb: Number(node.availableRamMb || 0),
        verifiedAccelerators,
      };
      nodes = [...nodes.filter((n) => n.nodeId !== entry.nodeId), entry];
      return entry;
    }
  };
}

