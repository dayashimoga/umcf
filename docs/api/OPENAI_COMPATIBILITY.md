# UMCF Local AI Engine — OpenAI API Compatibility Guide

The Universal Mobile Compute Fabric (UMCF) provides an OpenAI-compatible HTTP/SSE API server running locally on each node:

- **Base URL**: `http://127.0.0.1:11435/v1`
- **Supported Enclaves**: `StrictLocalOnly`, `TrustedPersonalDevicesOnly`, `ClusterEncrypted`
- **Authentication**: Optional Bearer token or `X-UMCF-Privacy-Mode` header.

---

## 1. Supported Endpoints

| Endpoint | Method | Streaming | Description |
|---|---|---|---|
| `/v1/models` | `GET` | No | Lists locally resident and cluster-available GGUF models. |
| `/v1/models/{model}` | `GET` | No | Retrieves metadata, layer counts, and privacy status for a specific model. |
| `/v1/chat/completions` | `POST` | Yes (`text/event-stream`) | Multi-turn conversational completions with model-specific chat templates. |
| `/v1/completions` | `POST` | Yes (`text/event-stream`) | Legacy text prompt completions. |
| `/v1/embeddings` | `POST` | No | Generates L2-normalized float embeddings for vector search / RAG pipelines. |

---

## 2. Privacy Mode Header (`X-UMCF-Privacy-Mode`)

Every request may specify the desired privacy and data sovereignty guarantee:

- `strict-local` (Default): Guaranteed zero outbound network packets. Inference executes strictly on local hardware (CPU/GPU/NPU). If a model exceeds local memory, the server responds with `422 Unprocessable Entity` rather than routing activations over the network.
- `trusted-devices`: Allows distributed layer partitioning only across devices verified and signed by the user's primary key.
- `cluster-encrypted`: Allows cluster-wide mesh inference with ephemeral per-session X25519 key exchange and ChaCha20-Poly1305 encrypted activations.

---

## 3. Python SDK Usage (`openai>=1.0.0`)

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://127.0.0.1:11435/v1",
    api_key="umcf-local-session",
    default_headers={"X-UMCF-Privacy-Mode": "strict-local"}
)

# Streaming Chat Completion
stream = client.chat.completions.create(
    model="Llama-3-8B-Instruct-Q4",
    messages=[
        {"role": "system", "content": "You are a private AI assistant."},
        {"role": "user", "content": "Explain zero-knowledge proofs in one sentence."}
    ],
    stream=True
)

for chunk in stream:
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="", flush=True)
print()
```

---

## 4. TypeScript / Node.js SDK Usage (`openai>=4.0.0`)

```typescript
import OpenAI from 'openai';

const client = new OpenAI({
  baseURL: 'http://127.0.0.1:11435/v1',
  apiKey: 'umcf-local-session',
  defaultHeaders: { 'X-UMCF-Privacy-Mode': 'strict-local' },
});

async function main() {
  const completion = await client.chat.completions.create({
    model: 'Llama-3-8B-Instruct-Q4',
    messages: [{ role: 'user', content: 'Hello from TypeScript!' }],
  });

  console.log(completion.choices[0].message.content);
}

main();
```

---

## 5. Embeddings for Local RAG Pipelines

```python
response = client.embeddings.create(
    model="bge-small-en-v1.5",
    input=["Distributed AI on personal devices", "Private AI enclaves"]
)

vec1 = response.data[0].embedding
vec2 = response.data[1].embedding
print(f"Dimension: {len(vec1)}, Sample: {vec1[:3]}")
```
