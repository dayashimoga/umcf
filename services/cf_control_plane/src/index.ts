// Cloudflare Workers Control Plane entrypoint for UMCF
export interface ControlPlaneEnvelope {
  plane: "ControlPlane";
  kind: string;
  senderNodeId: string;
  payload?: unknown;
}

export const MAX_CONTROL_PLANE_BYTES = 65536;

export default {
  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);

    if (request.method === "GET" && url.pathname === "/healthz") {
      return Response.json({
        service: "cf_control_plane",
        status: "healthy",
        protocol: "1.0.0"
      });
    }

    if (request.method === "POST" && url.pathname === "/v1/control/envelope") {
      const text = await request.text();
      const bytes = new TextEncoder().encode(text).byteLength;
      if (bytes > MAX_CONTROL_PLANE_BYTES) {
        return Response.json(
          { error: `Payload exceeds ${MAX_CONTROL_PLANE_BYTES} bytes control-plane ceiling` },
          { status: 413 }
        );
      }
      try {
        const body = JSON.parse(text) as ControlPlaneEnvelope;
        if (body.plane !== "ControlPlane") {
          return Response.json(
            { error: "Only ControlPlane envelopes are allowed" },
            { status: 422 }
          );
        }
        return Response.json({ accepted: true, kind: body.kind });
      } catch {
        return Response.json({ error: "Invalid JSON" }, { status: 400 });
      }
    }

    return Response.json({ error: "Not Found" }, { status: 404 });
  }
};
