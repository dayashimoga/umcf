<script>
  import { createClusterDashboardState, PROTOCOL_VERSION } from "./cluster-store.svelte.js";

  const dashboard = createClusterDashboardState("umcf-edge-cluster");
  let role = $state(dashboard.localRole);
  let sharing = $state(dashboard.sharingEnabled);
  let privateAi = $state(dashboard.privateAiLocalOnly);

  function cycleRole() {
    const next = role === "Hybrid" ? "Master" : role === "Master" ? "Worker" : "Hybrid";
    role = dashboard.setRole(next);
  }

  function toggleSharing() {
    sharing = !sharing;
    dashboard.updateConsent({ enabled: sharing });
  }

  function togglePrivateAi() {
    privateAi = !privateAi;
    dashboard.setPrivateAiMode(privateAi);
  }
</script>

<main class="umcf-dashboard" aria-label="Universal Mobile Compute Fabric Web Console">
  <header>
    <h1>Universal Mobile Compute Fabric (UMCF)</h1>
    <p>Protocol v{PROTOCOL_VERSION} — Decentralized Master/Worker Compute Plane</p>
  </header>

  <section aria-labelledby="node-controls-heading">
    <h2 id="node-controls-heading">Local Node Governance & Consent</h2>
    <button type="button" onclick={cycleRole}>Role: {role}</button>
    <button type="button" onclick={toggleSharing}>
      Resource Sharing: {sharing ? "Enabled (Consented)" : "Disabled (Default Safe)"}
    </button>
    <button type="button" onclick={togglePrivateAi}>
      AI Privacy Mode: {privateAi ? "Private Local-Only" : "Cluster Distributed"}
    </button>
  </section>
</main>

