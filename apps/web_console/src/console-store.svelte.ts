// Svelte 5 runes state store for UMCF Web Console
export interface NodeSummary {
  nodeId: string;
  role: "Master" | "Worker" | "Hybrid";
  availableRamMb: number;
}

export function createConsoleState(initialCluster = "umcf-mesh-default") {
  let clusterId = $state(initialCluster);
  let nodes = $state<NodeSummary[]>([]);
  let activeWorkloads = $state(0);

  let totalClusterRamMb = $derived(
    nodes.reduce((acc, curr) => acc + curr.availableRamMb, 0)
  );

  return {
    get clusterId() {
      return clusterId;
    },
    get nodes() {
      return nodes;
    },
    get activeWorkloads() {
      return activeWorkloads;
    },
    get totalClusterRamMb() {
      return totalClusterRamMb;
    },
    setClusterId(id: string) {
      clusterId = id;
    },
    registerNode(node: NodeSummary) {
      nodes = [...nodes.filter((n) => n.nodeId !== node.nodeId), node];
    },
    incrementWorkloads() {
      activeWorkloads += 1;
    }
  };
}
