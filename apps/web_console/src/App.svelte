<script lang="ts">
  import { createConsoleState } from "./console-store.svelte.js";

  const consoleState = createConsoleState("umcf-console-cluster");
  let cluster = $state(consoleState.clusterId);
  let workloads = $state(consoleState.activeWorkloads);

  function triggerTask() {
    consoleState.incrementWorkloads();
    workloads = consoleState.activeWorkloads;
  }
</script>

<main aria-label="UMCF Web Console">
  <header>
    <h1>UMCF Decentralized Compute Console</h1>
    <p>Cluster: {cluster} | Active Workloads: {workloads}</p>
  </header>
  <section>
    <button type="button" onclick={triggerTask}>Schedule Benchmark Workload</button>
  </section>
</main>
