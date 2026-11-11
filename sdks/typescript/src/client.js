// client.js — Universal zero-dependency JavaScript UMCF Client for Node.js >= 20, Deno, Bun, and Browser.

export class UmcfError extends Error {
  constructor(status, message, problem) {
    super(message);
    this.name = "UmcfError";
    this.status = status;
    this.problem = problem;
  }
}

export class UmcfClient {
  constructor(config = {}) {
    this.endpoint = (config.endpoint ?? "http://127.0.0.1:8080").replace(/\/+$/, "");
    this.apiKey = config.apiKey;
    this.timeoutMs = config.timeoutMs ?? 30000;
  }

  async request(path, init = {}) {
    const url = `${this.endpoint}${path}`;
    const headers = new Headers(init.headers || {});

    if (this.apiKey && !headers.has("Authorization")) {
      headers.set("Authorization", `Bearer ${this.apiKey}`);
    }
    if (!headers.has("Accept")) {
      headers.set("Accept", "application/json, application/problem+json");
    }

    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeoutMs);

    try {
      const response = await fetch(url, {
        ...init,
        headers,
        signal: controller.signal,
      });

      if (response.ok) {
        if (response.headers.get("content-type")?.includes("application/octet-stream")) {
          const buffer = await response.arrayBuffer();
          return new Uint8Array(buffer);
        }
        return await response.json();
      }

      let problem;
      let errorMsg = `HTTP ${response.status} ${response.statusText}`;

      try {
        problem = await response.json();
        errorMsg = problem.detail || problem.title || errorMsg;
      } catch {
        const text = await response.text();
        if (text) errorMsg = text;
      }

      throw new UmcfError(response.status, errorMsg, problem);
    } finally {
      clearTimeout(timer);
    }
  }

  cluster = {
    listNodes: async () => this.request("/api/v1/cluster/nodes"),
    getTelemetry: async () => this.request("/api/v1/cluster/telemetry"),
    pair: async (pairingToken, initiatorPublicKey) => {
      return this.request("/api/v1/cluster/pair", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ pairing_token: pairingToken, initiator_public_key: initiatorPublicKey }),
      });
    },
  };

  jobs = {
    submit: async (submission) => {
      return this.request("/api/v1/jobs", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(submission),
      });
    },
    get: async (jobId) => this.request(`/api/v1/jobs/${encodeURIComponent(jobId)}`),
    cancel: async (jobId) => {
      return this.request(`/api/v1/jobs/${encodeURIComponent(jobId)}/cancel`, {
        method: "POST",
      });
    },
    submitWasm: async (moduleCid, entrypoint, params = {}) => {
      return this.jobs.submit({
        job_type: "wasm",
        target_role: "Worker",
        payload: { module_cid: moduleCid, entrypoint, params },
      });
    },
  };

  ai = {
    listModels: async () => this.request("/api/v1/ai/models"),
    pullModel: async (modelName) => {
      return this.request("/api/v1/ai/models/pull", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ model: modelName }),
      });
    },
    chat: async (req) => {
      return this.request("/v1/chat/completions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ...req, stream: false }),
      });
    },
  };

  storage = {
    putObject: async (data) => {
      return this.request("/api/v1/storage/objects", {
        method: "PUT",
        headers: { "Content-Type": "application/octet-stream" },
        body: data,
      });
    },
    getObject: async (cid) => this.request(`/api/v1/storage/objects/${encodeURIComponent(cid)}`),
    pin: async (req) => {
      return this.request("/api/v1/storage/pins", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(req),
      });
    },
  };

  gpu = {
    render: async (tiles) => {
      return this.request("/api/v1/gpu/render", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tiles }),
      });
    },
  };

  media = {
    transcode: async (segments) => {
      return this.request("/api/v1/media/transcode", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ segments }),
      });
    },
  };
}

