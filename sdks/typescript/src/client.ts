// client.ts — Universal zero-dependency TypeScript/JavaScript UMCF Client

import type {
  ChatRequest,
  ChatResponse,
  ClusterTelemetry,
  JobEventRecord,
  JobRecord,
  JobSubmission,
  ModelInfo,
  NodeDescriptor,
  PinRequest,
  PinResponse,
  Rfc9457ProblemDetails,
  StorageIngestResponse,
} from "./types.js";

export class UmcfError extends Error {
  public readonly status: number;
  public readonly problem?: Rfc9457ProblemDetails;

  constructor(status: number, message: string, problem?: Rfc9457ProblemDetails) {
    super(message);
    this.name = "UmcfError";
    this.status = status;
    this.problem = problem;
  }
}

export interface UmcfClientConfig {
  endpoint?: string;
  apiKey?: string;
  timeoutMs?: number;
}

export class UmcfClient {
  private readonly endpoint: string;
  private readonly apiKey?: string;
  private readonly timeoutMs: number;

  constructor(config: UmcfClientConfig = {}) {
    this.endpoint = (config.endpoint ?? "http://127.0.0.1:8080").replace(/\/+$/, "");
    this.apiKey = config.apiKey;
    this.timeoutMs = config.timeoutMs ?? 30000;
  }

  private async request<T>(path: string, init: RequestInit = {}): Promise<T> {
    const url = `${this.endpoint}${path}`;
    const headers = new Headers(init.headers);

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
          return new Uint8Array(buffer) as unknown as T;
        }
        return (await response.json()) as T;
      }

      // Handle RFC 9457 Problem Details
      let problem: Rfc9457ProblemDetails | undefined;
      let errorMsg = `HTTP ${response.status} ${response.statusText}`;

      try {
        problem = (await response.json()) as Rfc9457ProblemDetails;
        errorMsg = problem.detail || problem.title || errorMsg;
      } catch {
        // Fallback to text
        const text = await response.text();
        if (text) errorMsg = text;
      }

      throw new UmcfError(response.status, errorMsg, problem);
    } finally {
      clearTimeout(timer);
    }
  }

  // --- Cluster Namespace ---
  public readonly cluster = {
    listNodes: async (): Promise<NodeDescriptor[]> => {
      return this.request<NodeDescriptor[]>("/api/v1/cluster/nodes");
    },
    getTelemetry: async (): Promise<ClusterTelemetry> => {
      return this.request<ClusterTelemetry>("/api/v1/cluster/telemetry");
    },
    pair: async (pairingToken: string, initiatorPublicKey: string) => {
      return this.request<{ status: string; peer_node_id: string }>("/api/v1/cluster/pair", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ pairing_token: pairingToken, initiator_public_key: initiatorPublicKey }),
      });
    },
  };

  // --- Jobs Namespace ---
  public readonly jobs = {
    submit: async (submission: JobSubmission): Promise<JobRecord> => {
      return this.request<JobRecord>("/api/v1/jobs", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(submission),
      });
    },
    get: async (jobId: string): Promise<JobRecord> => {
      return this.request<JobRecord>(`/api/v1/jobs/${encodeURIComponent(jobId)}`);
    },
    cancel: async (jobId: string): Promise<{ status: string }> => {
      return this.request<{ status: string }>(`/api/v1/jobs/${encodeURIComponent(jobId)}/cancel`, {
        method: "POST",
      });
    },
    submitWasm: async (moduleCid: string, entrypoint: string, params: Record<string, unknown> = {}) => {
      return this.jobs.submit({
        job_type: "wasm",
        target_role: "Worker",
        payload: { module_cid: moduleCid, entrypoint, params },
      });
    },
  };

  // --- AI Namespace ---
  public readonly ai = {
    listModels: async (): Promise<ModelInfo[]> => {
      return this.request<ModelInfo[]>("/api/v1/ai/models");
    },
    pullModel: async (modelName: string) => {
      return this.request<{ status: string; model: string }>("/api/v1/ai/models/pull", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ model: modelName }),
      });
    },
    chat: async (req: ChatRequest): Promise<ChatResponse> => {
      return this.request<ChatResponse>("/v1/chat/completions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ...req, stream: false }),
      });
    },
  };

  // --- Storage Namespace ---
  public readonly storage = {
    putObject: async (data: Uint8Array): Promise<StorageIngestResponse> => {
      return this.request<StorageIngestResponse>("/api/v1/storage/objects", {
        method: "PUT",
        headers: { "Content-Type": "application/octet-stream" },
        body: data,
      });
    },
    getObject: async (cid: string): Promise<Uint8Array> => {
      return this.request<Uint8Array>(`/api/v1/storage/objects/${encodeURIComponent(cid)}`);
    },
    pin: async (req: PinRequest): Promise<PinResponse> => {
      return this.request<PinResponse>("/api/v1/storage/pins", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(req),
      });
    },
  };

  // --- GPU & Media Namespace ---
  public readonly gpu = {
    render: async (tiles: number) => {
      return this.request<{ job_id: string; status: string; tiles: number }>("/api/v1/gpu/render", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tiles }),
      });
    },
  };

  public readonly media = {
    transcode: async (segments: number) => {
      return this.request<{ job_id: string; status: string; segments: number }>("/api/v1/media/transcode", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ segments }),
      });
    },
  };
}

