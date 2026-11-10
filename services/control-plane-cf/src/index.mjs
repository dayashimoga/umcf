// Cloudflare Workers + Durable Objects Control Plane for UMCF
// Enforces strict Control-Plane vs. Data-Plane isolation: bulk payloads (> 64 KiB) or
// data-plane message kinds are deterministically rejected with HTTP 413 / 422.

export const MAX_CONTROL_PLANE_PAYLOAD_BYTES = 65536;
export const DATA_PLANE_FORBIDDEN_KINDS = new Set([
  "TensorShardFrame",
  "InferenceActivationFrame",
  "RenderTileFrame",
  "MediaSegmentFrame",
  "RemoteStreamVideoFrame",
  "RemoteStreamInputFrame",
  "StorageChunkFrame"
]);

export function validateControlPlaneEnvelope(envelope, rawByteLength = 0) {
  if (rawByteLength > MAX_CONTROL_PLANE_PAYLOAD_BYTES) {
    return {
      ok: false,
      status: 413,
      error: `Payload size (${rawByteLength} bytes) exceeds 64 KiB control-plane ceiling`
    };
  }
  if (!envelope || typeof envelope !== "object") {
    return { ok: false, status: 400, error: "Malformed control-plane JSON envelope" };
  }
  if (envelope.plane !== "ControlPlane") {
    return {
      ok: false,
      status: 422,
      error: "Plane isolation violation: only ControlPlane messages are accepted on edge coordinator"
    };
  }
  if (DATA_PLANE_FORBIDDEN_KINDS.has(envelope.kind)) {
    return {
      ok: false,
      status: 422,
      error: `Message kind '${envelope.kind}' must use direct P2P QUIC DataPlane`
    };
  }
  if (typeof envelope.senderNodeId !== "string" || !envelope.senderNodeId.startsWith("did:umcf:")) {
    return { ok: false, status: 400, error: "Invalid senderNodeId DID format" };
  }
  return { ok: true, status: 200 };
}

export default {
  async fetch(request) {
    const url = new URL(request.url);
    if (request.method === "GET" && url.pathname === "/healthz") {
      return Response.json({
        status: "healthy",
        service: "umcf-control-plane-cf",
        protocol: "1.0.0",
        plane: "ControlPlane"
      });
    }

    if (request.method === "POST" && url.pathname === "/v1/control/envelope") {
      const text = await request.text();
      const byteLen = new TextEncoder().encode(text).byteLength;
      let parsed;
      try {
        parsed = JSON.parse(text);
      } catch {
        return Response.json({ error: "Invalid JSON" }, { status: 400 });
      }
      const verdict = validateControlPlaneEnvelope(parsed, byteLen);
      if (!verdict.ok) {
        return Response.json({ error: verdict.error }, { status: verdict.status });
      }
      return Response.json({
        accepted: true,
        kind: parsed.kind,
        senderNodeId: parsed.senderNodeId
      });
    }

    return Response.json({ error: "Not Found" }, { status: 404 });
  }
};

