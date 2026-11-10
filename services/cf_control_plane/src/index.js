// Cloudflare Workers Control Plane entrypoint for UMCF.
// Manages authentication, cluster routing, and Durable Object coordination.
// Strictly enforces zero-placeholder and default-deny invariants.

import { verifyRequestSignature } from "./crypto.js";
import { ClusterCoordinatorDO } from "./durable_object.js";

export { ClusterCoordinatorDO };

// In-memory fallback coordinators for local simulation / testing
const localCoordinators = new Map();

/**
 * Gets or creates an in-memory coordinator for a cluster ID
 * @param {string} clusterId
 * @returns {ClusterCoordinatorDO}
 */
export function getLocalCoordinator(clusterId) {
  let coord = localCoordinators.get(clusterId);
  if (!coord) {
    coord = new ClusterCoordinatorDO();
    coord.clusterId = clusterId;
    localCoordinators.set(clusterId, coord);
  }
  return coord;
}

/** Resets all in-memory local coordinators */
export function resetLocalCoordinators() {
  localCoordinators.clear();
}

/**
 * Handles an incoming HTTP request across Cloudflare Workers or local simulation
 * @param {Request} request
 * @param {any} [env]
 * @returns {Promise<Response>}
 */
export async function handleControlPlaneRequest(request, env) {
  const url = new URL(request.url);

  // 1. Healthcheck endpoint (Unauthenticated)
  if (request.method === "GET" && url.pathname === "/healthz") {
    return Response.json({
      service: "cf_control_plane",
      status: "healthy",
      protocol: "1.0.0",
      timestamp_utc: new Date().toISOString()
    });
  }

  // 2. Cluster API routing: /v1/clusters/:id/:action
  const clusterMatch = url.pathname.match(
    /^\/v1\/clusters\/([^/]+)\/(register|sync-delta|peers|ws)$/
  );

  if (!clusterMatch) {
    return Response.json(
      { code: "E4040", error: "Not Found: Invalid cluster API path" },
      { status: 404 }
    );
  }

  const clusterId = decodeURIComponent(clusterMatch[1] ?? "");
  const action = clusterMatch[2] ?? "";

  // Read request body bytes once
  const bodyBuffer = await request.arrayBuffer();
  const bodyBytes = new Uint8Array(bodyBuffer);

  // 3. Cryptographic authentication gate (Ed25519 signature & replay check)
  const auth = await verifyRequestSignature(
    request.method,
    url.pathname,
    request.headers,
    bodyBytes
  );

  if (!auth.valid) {
    return Response.json(
      {
        code: "E2001",
        error: auth.error ?? "Unauthorized: Invalid or expired Ed25519 signature"
      },
      { status: 401 }
    );
  }

  // 4. Resolve Durable Object coordinator for cluster
  let coordinator;
  const doNamespace = env?.CLUSTER_COORDINATOR ?? env?.CLUSTER_REGISTRY;

  if (doNamespace) {
    const doId = doNamespace.idFromName(clusterId);
    coordinator = doNamespace.get(doId);
  } else {
    // Local simulation fallback
    coordinator = getLocalCoordinator(clusterId);
  }

  // 5. Route dispatch
  try {
    switch (action) {
      case "register": {
        if (request.method !== "POST") {
          return Response.json({ error: "Method Not Allowed" }, { status: 405 });
        }
        const text = new TextDecoder().decode(bodyBytes);
        const data = JSON.parse(text);
        const result = coordinator.registerPeer({
          nodeId: data.node_id ?? auth.did,
          role: data.role ?? "Worker",
          rendezvousEndpoints: data.rendezvous_endpoints ?? [],
          capabilities: data.capabilities
        });
        return Response.json({
          status: result.status,
          cluster_id: clusterId,
          node_id: result.peer.nodeId,
          active_peer_count: result.activePeerCount,
          peer: result.peer
        });
      }

      case "sync-delta": {
        if (request.method !== "POST") {
          return Response.json({ error: "Method Not Allowed" }, { status: 405 });
        }
        const text = new TextDecoder().decode(bodyBytes);
        const delta = JSON.parse(text);
        const result = coordinator.syncDelta(delta);
        return Response.json({
          accepted: result.accepted,
          current_hlc: result.currentHlc,
          peer_count: result.peerCount
        });
      }

      case "peers": {
        if (request.method !== "GET") {
          return Response.json({ error: "Method Not Allowed" }, { status: 405 });
        }
        const peers = coordinator.getPeers();
        return Response.json({
          cluster_id: clusterId,
          peer_count: peers.length,
          peers
        });
      }

      case "ws": {
        if (request.headers.get("Upgrade")?.toLowerCase() !== "websocket") {
          return Response.json(
            { error: "Expected WebSocket upgrade request" },
            { status: 426 }
          );
        }

        // Cloudflare Workers WebSocketPair support
        if (typeof globalThis.WebSocketPair !== "undefined") {
          const webSocketPair = new globalThis.WebSocketPair();
          const [clientWs, serverWs] = Object.values(webSocketPair);
          coordinator.handleWebSocket(serverWs, auth.did);
          return new Response(null, {
            status: 101,
            webSocket: clientWs
          });
        }

        return Response.json(
          { error: "WebSocket upgrade requires WebSocketPair or local simulation server" },
          { status: 501 }
        );
      }

      default:
        return Response.json({ error: "Unknown action" }, { status: 404 });
    }
  } catch (err) {
    return Response.json(
      { code: "E5000", error: `Internal coordinator error: ${err?.message ?? err}` },
      { status: 500 }
    );
  }
}

export default {
  async fetch(request, env) {
    return handleControlPlaneRequest(request, env);
  }
};
