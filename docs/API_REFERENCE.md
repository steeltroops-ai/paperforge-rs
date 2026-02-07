# PaperForge-rs: API Reference V2

**Version**: 2.0  
**Base URL**: `https://api.paperforge.example.com/v2`  
**Format**: JSON

---

## Authentication

All API requests require authentication:

```http
Authorization: Bearer <api_key>
X-Tenant-ID: <tenant_uuid>
X-Request-ID: <correlation_id>  # Optional, auto-generated if missing
```

### Rate Limits

| Plan       | Requests/Second | Burst |
| ---------- | --------------- | ----- |
| Free       | 10              | 20    |
| Standard   | 50              | 100   |
| Enterprise | 500             | 1000  |

Rate limit headers in response:

```http
X-RateLimit-Limit: 50
X-RateLimit-Remaining: 49
X-RateLimit-Reset: 1707334800
```

---

## Endpoints

### Health & Status

#### GET /health

Liveness probe (no auth required).

**Response**: `200 OK`

```json
{
  "status": "healthy"
}
```

#### GET /ready

Readiness probe with dependency checks.

**Response**: `200 OK` or `503 Service Unavailable`

```json
{
  "status": "ready",
  "checks": {
    "database": { "status": "up", "latency_ms": 5 },
    "redis": { "status": "up", "latency_ms": 2 },
    "embedding": { "status": "up", "latency_ms": 150 }
  }
}
```

---

### Ingestion API

#### POST /papers

Create a new paper and start async ingestion.

**Request**:

```json
{
  "idempotency_key": "arxiv:2301.12345",
  "paper": {
    "title": "Attention Is All You Need",
    "abstract": "The dominant sequence transduction models are based on complex recurrent or convolutional neural networks...",
    "source": "arxiv",
    "external_id": "1706.03762",
    "published_at": "2017-06-12T00:00:00Z",
    "metadata": {
      "authors": ["Vaswani", "Shazeer", "Parmar"],
      "keywords": ["transformers", "attention", "neural networks"],
      "doi": "10.48550/arXiv.1706.03762"
    }
  },
  "options": {
    "embedding_model": "text-embedding-ada-002",
    "chunk_strategy": "semantic",
    "chunk_size": 512,
    "chunk_overlap": 50
  }
}
```

**Response**: `202 Accepted`

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "pending",
  "estimated_completion_ms": 5000,
  "poll_url": "/v2/jobs/550e8400-e29b-41d4-a716-446655440000"
}
```

**Errors**:

- `400 Bad Request`: Invalid input
- `409 Conflict`: Duplicate idempotency_key (returns existing job)
- `429 Too Many Requests`: Rate limit exceeded

#### GET /jobs/{job_id}

Get job status.

**Response**: `200 OK`

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "paper_id": "123e4567-e89b-12d3-a456-426614174000",
  "chunks_created": 12,
  "chunks_total": 12,
  "processing_time_ms": 3200,
  "started_at": "2026-02-07T19:29:57Z",
  "completed_at": "2026-02-07T19:30:00Z"
}
```

**Status Values**:

- `pending`: Queued for processing
- `chunking`: Splitting text into chunks
- `embedding`: Generating embeddings
- `indexing`: Writing to database
- `completed`: Successfully finished
- `failed`: Error occurred (see `error_message`)

#### GET /papers/{paper_id}

Get paper details.

**Response**: `200 OK`

```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "title": "Attention Is All You Need",
  "abstract": "The dominant sequence...",
  "source": "arxiv",
  "external_id": "1706.03762",
  "published_at": "2017-06-12T00:00:00Z",
  "metadata": {
    "authors": ["Vaswani", "Shazeer", "Parmar"],
    "keywords": ["transformers", "attention"]
  },
  "chunk_count": 12,
  "created_at": "2026-02-07T19:30:00Z"
}
```

#### DELETE /papers/{paper_id}

Delete a paper and all associated chunks.

**Response**: `204 No Content`

---

### Search API

#### POST /search

Perform semantic/hybrid search.

**Request**:

```json
{
  "query": "transformer architecture attention mechanisms",
  "options": {
    "mode": "hybrid",
    "limit": 20,
    "offset": 0,
    "rerank": true,
    "rerank_model": "cross-encoder",
    "min_score": 0.5,
    "temporal_weight": "neutral",
    "filters": {
      "source": ["arxiv", "pubmed"],
      "published_after": "2020-01-01",
      "published_before": "2026-01-01",
      "metadata.keywords": ["transformers"]
    }
  }
}
```

**Search Modes**:

- `vector`: Pure semantic similarity
- `bm25`: Pure keyword matching
- `hybrid`: RRF fusion of vector + BM25 (default)

**Temporal Weights**:

- `recent`: Boost recent papers
- `seminal`: Boost highly-cited older papers
- `neutral`: No temporal adjustment (default)

**Response**: `200 OK`

```json
{
  "query": "transformer architecture attention mechanisms",
  "mode": "hybrid",
  "total_results": 156,
  "results": [
    {
      "chunk_id": "abc123-...",
      "paper_id": "def456-...",
      "paper_title": "Attention Is All You Need",
      "content": "The Transformer follows this overall architecture using stacked self-attention...",
      "score": 0.92,
      "chunk_index": 3,
      "highlights": [
        { "text": "Transformer", "offset": 4 },
        { "text": "self-attention", "offset": 54 }
      ]
    }
  ],
  "facets": {
    "sources": { "arxiv": 120, "pubmed": 36 },
    "years": { "2023": 45, "2022": 38, "2021": 33 }
  },
  "processing_time_ms": 87
}
```

#### POST /search/batch

Batch search for multiple queries.

**Request**:

```json
{
  "queries": [
    { "query": "transformer attention", "limit": 10 },
    { "query": "BERT language model", "limit": 10 }
  ],
  "options": {
    "mode": "hybrid"
  }
}
```

**Response**: `200 OK`

```json
{
  "results": [
    {"query": "transformer attention", "results": [...]},
    {"query": "BERT language model", "results": [...]}
  ],
  "processing_time_ms": 145
}
```

---

### Intelligence API (Context Engine)

#### POST /intelligence/search

Intelligent search with context stitching and reasoning.

**Request**:

```json
{
  "query": "How does the attention mechanism in transformers compare to LSTM gating?",
  "session_id": "session-123",
  "options": {
    "mode": "deep",
    "max_hops": 2,
    "temporal_weight": "neutral",
    "include_reasoning": true,
    "include_synthesis": true,
    "limit": 20
  }
}
```

**Intelligence Modes**:

- `quick`: Vector search + rerank only (fastest)
- `standard`: Hybrid + citation boost
- `deep`: Multi-hop reasoning
- `synthesis`: Full LLM synthesis (slowest)

**Response**: `200 OK`

```json
{
  "query": "How does the attention mechanism...",
  "session_id": "session-123",
  "query_understanding": {
    "intent": "comparison_query",
    "entities": [
      { "text": "attention mechanism", "type": "concept" },
      { "text": "transformers", "type": "model" },
      { "text": "LSTM gating", "type": "concept" }
    ],
    "expanded_terms": ["self-attention", "scaled dot-product", "forget gate"]
  },
  "results": [
    {
      "chunk_id": "...",
      "paper_title": "Attention Is All You Need",
      "content": "...",
      "score": 0.95,
      "citation_boost": 0.12
    }
  ],
  "context": {
    "windows": [
      {
        "paper_id": "...",
        "paper_title": "Attention Is All You Need",
        "content": "Full stitched context from multiple chunks...",
        "chunk_range": [2, 5],
        "relevance_score": 0.92
      }
    ],
    "cross_references": [
      {
        "from_window": 0,
        "to_window": 1,
        "type": "citation"
      }
    ],
    "total_tokens": 2048
  },
  "reasoning": {
    "hops": [
      {
        "query": "attention mechanism in transformers",
        "facts_extracted": 5,
        "next_query": "LSTM gating mechanisms"
      },
      {
        "query": "LSTM gating mechanisms",
        "facts_extracted": 4,
        "next_query": null
      }
    ]
  },
  "synthesis": {
    "answer": "The attention mechanism in Transformers differs fundamentally from LSTM gating. While LSTM gates (forget, input, output) operate sequentially and control information flow through cell states, Transformer attention computes parallel relationships between all positions using scaled dot-product attention. Key differences include:\n\n1. **Parallelization**: Attention allows O(1) sequential operations vs O(n) for LSTMs [1]\n2. **Long-range dependencies**: Attention directly connects any positions [2]\n3. **Computational complexity**: O(n^2) for attention vs O(n) for LSTMs [1]\n\n[1] Attention Is All You Need\n[2] BERT: Pre-training of Deep Bidirectional Transformers",
    "citations": [
      { "index": 1, "paper_id": "...", "title": "Attention Is All You Need" },
      { "index": 2, "paper_id": "...", "title": "BERT: Pre-training..." }
    ],
    "confidence": 0.85
  },
  "processing_time_ms": 1250
}
```

---

### Session API

#### POST /sessions

Create a new session.

**Request**:

```json
{
  "metadata": {
    "user_agent": "PaperForge-SDK/1.0",
    "research_topic": "machine learning"
  }
}
```

**Response**: `201 Created`

```json
{
  "session_id": "session-abc123",
  "expires_at": "2026-02-07T20:30:00Z"
}
```

#### GET /sessions/{session_id}

Get session state.

**Response**: `200 OK`

```json
{
  "session_id": "session-abc123",
  "queries": [
    {
      "query": "transformer attention",
      "timestamp": "2026-02-07T19:30:00Z",
      "clicked_results": ["chunk-123", "chunk-456"]
    }
  ],
  "preferred_topics": {
    "transformers": 0.8,
    "attention": 0.7
  },
  "viewed_papers": ["paper-abc", "paper-def"],
  "expires_at": "2026-02-07T20:30:00Z"
}
```

#### POST /sessions/{session_id}/events

Track user events.

**Request**:

```json
{
  "event": "click",
  "data": {
    "chunk_id": "chunk-123",
    "paper_id": "paper-abc"
  }
}
```

**Response**: `204 No Content`

---

### Citation API

#### GET /papers/{paper_id}/citations

Get citation graph for a paper.

**Response**: `200 OK`

```json
{
  "paper_id": "123e4567-...",
  "paper_title": "Attention Is All You Need",
  "citations": {
    "outgoing": [
      {
        "cited_paper_id": "...",
        "cited_paper_title": "Sequence to Sequence Learning",
        "context": "...builds upon prior work in neural machine translation..."
      }
    ],
    "incoming": [
      {
        "citing_paper_id": "...",
        "citing_paper_title": "BERT: Pre-training...",
        "context": "...following the Transformer architecture from..."
      }
    ]
  },
  "stats": {
    "outgoing_count": 35,
    "incoming_count": 15420
  }
}
```

#### POST /citations/traverse

Multi-hop citation traversal.

**Request**:

```json
{
  "seed_papers": ["paper-123", "paper-456"],
  "direction": "both",
  "max_hops": 2,
  "limit": 50
}
```

**Response**: `200 OK`

```json
{
  "seed_papers": ["paper-123", "paper-456"],
  "papers": [
    {
      "paper_id": "...",
      "title": "...",
      "hop_distance": 1,
      "propagation_score": 0.85
    }
  ],
  "graph": {
    "nodes": [...],
    "edges": [...]
  }
}
```

---

## Error Responses

All errors follow this format:

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Title must be between 1 and 1000 characters",
    "details": {
      "field": "paper.title",
      "constraint": "length",
      "min": 1,
      "max": 1000
    },
    "request_id": "req-abc123"
  }
}
```

### Error Codes

| HTTP Status | Code                  | Description                |
| ----------- | --------------------- | -------------------------- |
| 400         | `VALIDATION_ERROR`    | Invalid request body       |
| 400         | `MISSING_FIELD`       | Required field missing     |
| 401         | `UNAUTHORIZED`        | Invalid or missing API key |
| 403         | `FORBIDDEN`           | Insufficient permissions   |
| 404         | `NOT_FOUND`           | Resource not found         |
| 409         | `CONFLICT`            | Duplicate resource         |
| 429         | `RATE_LIMITED`        | Too many requests          |
| 500         | `INTERNAL_ERROR`      | Server error               |
| 502         | `UPSTREAM_ERROR`      | External service error     |
| 503         | `SERVICE_UNAVAILABLE` | Service temporarily down   |

---

## Webhooks (Optional)

Register webhooks for async notifications:

#### POST /webhooks

```json
{
  "url": "https://your-server.com/webhook",
  "events": ["paper.ingested", "paper.failed"],
  "secret": "your-webhook-secret"
}
```

**Webhook Payload**:

```json
{
  "event": "paper.ingested",
  "timestamp": "2026-02-07T19:30:00Z",
  "data": {
    "job_id": "...",
    "paper_id": "...",
    "chunks_created": 12
  },
  "signature": "sha256=..."
}
```

---

## SDKs

### Python

```bash
pip install paperforge-py
```

```python
from paperforge import PaperForge

client = PaperForge(api_key="pk_...", tenant_id="...")

# Ingest
job = client.papers.create(
    title="...",
    abstract="...",
    wait=True  # Block until completed
)

# Search
results = client.search("transformer attention", limit=20)
```

### Rust

```bash
cargo add paperforge-rs
```

```rust
use paperforge::PaperForge;

let client = PaperForge::new("pk_...", "tenant-id");

// Ingest
let job = client.papers().create(CreatePaperRequest {
    title: "...".into(),
    abstract_text: "...".into(),
    ..Default::default()
}).await?;

// Search
let results = client.search("transformer attention")
    .limit(20)
    .send()
    .await?;
```
