"""client.py — Polyglot synchronous and asynchronous Python UMCF Client."""

import json
import urllib.error
import urllib.parse
import urllib.request
from typing import Any, AsyncIterator, Dict, List, Optional, Union

from .errors import ForbiddenError, NotFoundError, RateLimitError, UnauthorizedError, UmcfError
from .models import (
    ChatMessage,
    ClusterTelemetry,
    JobRecord,
    ModelInfo,
    NodeDescriptor,
    PinResponse,
    StorageIngestResponse,
)


class UmcfClient:
    """Synchronous client for the Universal Mobile Compute Fabric API."""

    def __init__(
        self,
        endpoint: str = "http://127.0.0.1:8080",
        api_key: Optional[str] = None,
        timeout: float = 30.0,
    ):
        self.endpoint = endpoint.rstrip("/")
        self.api_key = api_key
        self.timeout = timeout

        self.cluster = _ClusterNamespace(self)
        self.jobs = _JobsNamespace(self)
        self.ai = _AiNamespace(self)
        self.storage = _StorageNamespace(self)
        self.gpu = _GpuNamespace(self)
        self.media = _MediaNamespace(self)

    def _request(
        self,
        method: str,
        path: str,
        body: Optional[Union[bytes, Dict[str, Any]]] = None,
        content_type: str = "application/json",
    ) -> Any:
        url = f"{self.endpoint}{path}"
        headers = {
            "Accept": "application/json, application/problem+json",
        }
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"

        encoded_data: Optional[bytes] = None
        if body is not None:
            if isinstance(body, bytes):
                encoded_data = body
                headers["Content-Type"] = content_type
            else:
                encoded_data = json.dumps(body).encode("utf-8")
                headers["Content-Type"] = "application/json"

        req = urllib.request.Request(url, data=encoded_data, headers=headers, method=method)

        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                resp_content_type = resp.headers.get("Content-Type", "")
                raw = resp.read()
                if "application/octet-stream" in resp_content_type:
                    return raw
                if not raw:
                    return {}
                return json.loads(raw.decode("utf-8"))
        except urllib.error.HTTPError as e:
            raw_err = e.read()
            problem: Dict[str, Any] = {}
            error_msg = f"HTTP {e.code}: {e.reason}"
            try:
                problem = json.loads(raw_err.decode("utf-8"))
                error_msg = problem.get("detail") or problem.get("title") or error_msg
            except Exception:
                pass

            if e.code == 401:
                raise UnauthorizedError(401, error_msg, problem) from e
            elif e.code == 403:
                raise ForbiddenError(403, error_msg, problem) from e
            elif e.code == 404:
                raise NotFoundError(404, error_msg, problem) from e
            elif e.code == 429:
                raise RateLimitError(error_msg, 1000) from e
            else:
                raise UmcfError(e.code, error_msg, problem) from e


class _ClusterNamespace:
    def __init__(self, client: UmcfClient):
        self._c = client

    def list_nodes(self) -> List[NodeDescriptor]:
        data = self._c._request("GET", "/api/v1/cluster/nodes")
        return [
            NodeDescriptor(
                node_id=n["node_id"],
                role=n["role"],
                os=n["os"],
                cpu_cores=n["cpu_cores"],
                ram_total_mb=n["ram_total_mb"],
                ram_available_mb=n["ram_available_mb"],
                is_ac_powered=n["is_ac_powered"],
                status=n["status"],
                gpu_model=n.get("gpu_model"),
            )
            for n in data
        ]

    def get_telemetry(self) -> ClusterTelemetry:
        t = self._c._request("GET", "/api/v1/cluster/telemetry")
        return ClusterTelemetry(
            total_nodes=t["total_nodes"],
            active_workers=t["active_workers"],
            aggregate_cpu_cores=t["aggregate_cpu_cores"],
            aggregate_ram_gb=t["aggregate_ram_gb"],
            cluster_storage_used_gb=t["cluster_storage_used_gb"],
            cluster_storage_capacity_gb=t["cluster_storage_capacity_gb"],
            network_mesh_p95_rtt_ms=t["network_mesh_p95_rtt_ms"],
        )

    def pair(self, pairing_token: str, initiator_public_key: str) -> Dict[str, Any]:
        return self._c._request(
            "POST",
            "/api/v1/cluster/pair",
            {"pairing_token": pairing_token, "initiator_public_key": initiator_public_key},
        )


class _JobsNamespace:
    def __init__(self, client: UmcfClient):
        self._c = client

    def submit(
        self,
        job_type: str,
        payload: Dict[str, Any],
        target_role: Optional[str] = None,
        priority: int = 10,
    ) -> JobRecord:
        body = {
            "job_type": job_type,
            "payload": payload,
            "target_role": target_role,
            "priority": priority,
        }
        res = self._c._request("POST", "/api/v1/jobs", body)
        return JobRecord(
            job_id=res["job_id"],
            job_type=res["job_type"],
            status=res["status"],
            progress_pct=res["progress_pct"],
            assigned_node=res["assigned_node"],
            payload=res.get("payload", {}),
            created_at_ms=res["created_at_ms"],
        )

    def get(self, job_id: str) -> JobRecord:
        res = self._c._request("GET", f"/api/v1/jobs/{urllib.parse.quote(job_id)}")
        return JobRecord(
            job_id=res["job_id"],
            job_type=res["job_type"],
            status=res["status"],
            progress_pct=res["progress_pct"],
            assigned_node=res["assigned_node"],
            payload=res.get("payload", {}),
            created_at_ms=res["created_at_ms"],
        )

    def cancel(self, job_id: str) -> Dict[str, Any]:
        return self._c._request("POST", f"/api/v1/jobs/{urllib.parse.quote(job_id)}/cancel")

    def submit_wasm(
        self, module_cid: str, entrypoint: str, params: Optional[Dict[str, Any]] = None
    ) -> JobRecord:
        return self.submit(
            job_type="wasm",
            payload={"module_cid": module_cid, "entrypoint": entrypoint, "params": params or {}},
            target_role="Worker",
        )


class _AiNamespace:
    def __init__(self, client: UmcfClient):
        self._c = client

    def list_models(self) -> List[ModelInfo]:
        data = self._c._request("GET", "/api/v1/ai/models")
        return [
            ModelInfo(
                name=m["name"],
                architecture=m["architecture"],
                quantization=m["quantization"],
                context_length=m["context_length"],
                vram_required_mb=m["vram_required_mb"],
                is_locally_resident=m["is_locally_resident"],
            )
            for m in data
        ]

    def pull_model(self, model_name: str) -> Dict[str, Any]:
        return self._c._request("POST", "/api/v1/ai/models/pull", {"model": model_name})

    def chat(self, model: str, messages: List[ChatMessage], max_tokens: int = 256) -> Dict[str, Any]:
        body = {
            "model": model,
            "messages": [{"role": m.role, "content": m.content} for m in messages],
            "stream": False,
            "max_tokens": max_tokens,
        }
        return self._c._request("POST", "/v1/chat/completions", body)


class _StorageNamespace:
    def __init__(self, client: UmcfClient):
        self._c = client

    def put_object(self, data: bytes) -> StorageIngestResponse:
        res = self._c._request("PUT", "/api/v1/storage/objects", data, content_type="application/octet-stream")
        return StorageIngestResponse(
            root_cid=res["root_cid"],
            size_bytes=res["size_bytes"],
            chunk_count=res["chunk_count"],
            deduplicated_chunks=res["deduplicated_chunks"],
        )

    def get_object(self, cid: str) -> bytes:
        return self._c._request("GET", f"/api/v1/storage/objects/{urllib.parse.quote(cid)}")

    def pin(self, cid: str, mode: str = "permanent", ttl_seconds: Optional[int] = None) -> PinResponse:
        body = {"cid": cid, "mode": mode, "ttl_seconds": ttl_seconds}
        res = self._c._request("POST", "/api/v1/storage/pins", body)
        return PinResponse(
            status=res["status"],
            cid=res["cid"],
            mode=res["mode"],
            hydration_status=res["hydration_status"],
        )


class _GpuNamespace:
    def __init__(self, client: UmcfClient):
        self._c = client

    def render(self, tiles: int) -> Dict[str, Any]:
        return self._c._request("POST", "/api/v1/gpu/render", {"tiles": tiles})


class _MediaNamespace:
    def __init__(self, client: UmcfClient):
        self._c = client

    def transcode(self, segments: int) -> Dict[str, Any]:
        return self._c._request("POST", "/api/v1/media/transcode", {"segments": segments})
