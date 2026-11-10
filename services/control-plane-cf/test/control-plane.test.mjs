import test from "node:test";
import assert from "node:assert/strict";
import worker, { validateControlPlaneEnvelope, MAX_CONTROL_PLANE_PAYLOAD_BYTES } from "../src/index.mjs";

test("[UNIT] control-plane accepts valid ControlPlane heartbeat and rejects DataPlane frames", () => {
  const valid = validateControlPlaneEnvelope(
    {
      plane: "ControlPlane",
      kind: "Heartbeat",
      senderNodeId: "did:umcf:0123456789abcdef0123456789abcdef"
    },
    120
  );
  assert.equal(valid.ok, true);
  assert.equal(valid.status, 200);

  const dataFrame = validateControlPlaneEnvelope(
    {
      plane: "ControlPlane",
      kind: "TensorShardFrame",
      senderNodeId: "did:umcf:0123456789abcdef0123456789abcdef"
    },
    512
  );
  assert.equal(dataFrame.ok, false);
  assert.equal(dataFrame.status, 422);

  const oversized = validateControlPlaneEnvelope(
    {
      plane: "ControlPlane",
      kind: "CapabilityReport",
      senderNodeId: "did:umcf:0123456789abcdef0123456789abcdef"
    },
    MAX_CONTROL_PLANE_PAYLOAD_BYTES + 10
  );
  assert.equal(oversized.ok, false);
  assert.equal(oversized.status, 413);
});

test("[INTEGRATION] worker fetch handler responds to /healthz and /v1/control/envelope", async () => {
  const healthRes = await worker.fetch(new Request("https://edge.umcf.local/healthz"));
  assert.equal(healthRes.status, 200);
  const healthBody = await healthRes.json();
  assert.equal(healthBody.status, "healthy");

  const postRes = await worker.fetch(
    new Request("https://edge.umcf.local/v1/control/envelope", {
      method: "POST",
      body: JSON.stringify({
        plane: "ControlPlane",
        kind: "CapabilityReport",
        senderNodeId: "did:umcf:abcdef0123456789abcdef0123456789"
      })
    })
  );
  assert.equal(postRes.status, 200);
  const postBody = await postRes.json();
  assert.equal(postBody.accepted, true);
});

