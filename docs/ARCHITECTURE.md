# PaperForge-rs: System Architecture V2

**Version**: 2.0  
**Status**: Production Specification  
**Date**: 2026-02-07

---

## 1. Overview

PaperForge-rs V2 is a **production-grade research discovery microservice platform** that combines semantic search, citation graph traversal, and context-aware AI to power intelligent academic literature discovery.

### Key Capabilities

1. **Semantic Search** - Vector similarity search over research papers
2. **Hybrid Search** - Combined vector + BM25 with Reciprocal Rank Fusion
3. **Citation Graph** - 2-hop traversal with PageRank-inspired scoring
4. **Context Engine** - Augment-like intelligent retrieval with session memory
5. **Async Processing** - Event-driven ingestion via SQS
6. **Multi-Tenant** - Isolated data per tenant with rate limiting

---

## 2. System Topology

```
                                    ┌──────────────────────────────────────┐
                                    │            Client Layer              │
                                    │  [Web Client] [API Client] [SDK]     │
                                    └───────────────────┬──────────────────┘
                                                        │
                                    ┌───────────────────▼──────────────────┐
                                    │            Edge Layer                │
                                    │   [CloudFront] → [WAF] → [ALB]       │
                                    └───────────────────┬──────────────────┘
                                                        │
                    ┌───────────────────────────────────┼───────────────────────────────────┐
                    │                                   │                                   │
            ┌───────▼────────┐                 ┌────────▼─────────┐              ┌──────────▼─────────┐
            │  API Gateway   │                 │  API Gateway     │              │   API Gateway      │
            │   Instance 1   │                 │   Instance 2     │              │   Instance N       │
            └───────┬────────┘                 └────────┬─────────┘              └──────────┬─────────┘
                    │                                   │                                   │
                    └───────────────────┬───────────────┴───────────────┬───────────────────┘
                                        │                               │
                    ┌───────────────────▼───────────┐ ┌─────────────────▼──────────────────┐
                    │       Ingestion Service       │ │         Search Service             │
                    │   [Paper CRUD] [Chunking]     │ │   [Vector] [Hybrid] [Ranking]      │
                    └───────────────────┬───────────┘ └─────────────────┬──────────────────┘
                                        │                               │
                                        │                               │
                    ┌───────────────────▼───────────┐ ┌─────────────────▼──────────────────┐
                    │         SQS Queue             │ │        Context Engine              │
                    │   [Ingestion] [DLQ]           │ │   [Stitcher] [Reasoner] [LLM]      │
                    └───────────────────┬───────────┘ └────────────────────────────────────┘
                                        │
                    ┌───────────────────▼───────────┐
                    │      Embedding Worker         │
                    │   [Batch] [Multi-Provider]    │
                    └───────────────────┬───────────┘
                                        │
                    ┌───────────────────▼───────────────────────────────────────────────────┐
                    │                        Data Layer                                     │
                    │   [RDS Primary] ←→ [Read Replica 1] ←→ [Read Replica 2]               │
                    │                    [ElastiCache Redis]                                │
                    └───────────────────────────────────────────────────────────────────────┘
```

---

## 3. Service Decomposition

### 3.1 API Gateway Service

**Responsibility**: Authentication, authorization, rate limiting, request routing

```rust
// Core capabilities
- JWT/API Key validation
- Token bucket rate limiting (per tenant)
- Request/response logging
- Circuit breaker for downstream services
- Request correlation (X-Request-ID)
- OpenAPI documentation
```

**Scaling**: Horizontal (3-10 instances)  
**Protocol**: HTTP/REST external, gRPC internal

### 3.2 Ingestion Service

**Responsibility**: Paper CRUD, chunking, job orchestration

```rust
// Core capabilities
- Paper creation with idempotency
- Semantic chunking with overlap
- Job queue management (SQS)
- Progress tracking and status API
- Dead letter queue handling
```

**Scaling**: Horizontal (2-5 instances)  
**Protocol**: gRPC internal, SQS async

### 3.3 Search Service

**Responsibility**: Query embedding, vector search, ranking

```rust
// Core capabilities
- Query embedding (cached)
- Vector similarity search (pgvector)
- BM25 text search
- Hybrid search with RRF
- Cross-encoder reranking
- Query caching (Redis)
```

**Scaling**: Horizontal (5-20 instances)  
**Protocol**: gRPC internal

### 3.4 Context Engine Service

**Responsibility**: Intelligence layer, reasoning, synthesis

```rust
// Core capabilities
- Query parsing and intent classification
- Query expansion (synonyms, session context)
- Context stitching (merge related chunks)
- Multi-hop reasoning
- Session memory management
- LLM integration for synthesis
- Citation propagation scoring
```

**Scaling**: Horizontal (2-5 instances)  
**Protocol**: gRPC internal

### 3.5 Embedding Worker

**Responsibility**: Async embedding generation

```rust
// Core capabilities
- SQS consumer
- Multi-provider support (OpenAI, Anthropic, local)
- Batch embedding optimization
- Retry with exponential backoff
- Model versioning
```

**Scaling**: Autoscale (1-50 instances based on queue depth)  
**Protocol**: SQS consumer

---

## 4. Data Flow Patterns

### 4.1 Ingestion Flow (Async)

```
1. Client → Gateway: POST /v2/papers (with idempotency_key)
2. Gateway → Ingestion: Validate, create paper record
3. Ingestion → SQS: Enqueue embedding job
4. Ingestion → Client: Return job_id (202 Accepted, <100ms)
5. Worker ← SQS: Dequeue job
6. Worker → Embedder: Generate embeddings (batched)
7. Worker → RDS: Store chunks with embeddings
8. Worker → SNS: Publish completion event
9. Client → Gateway: GET /v2/jobs/{id} (poll for status)
```

### 4.2 Search Flow (Sync)

```
1. Client → Gateway: POST /v2/search
2. Gateway → Search: Forward authenticated request
3. Search → Redis: Check cache
4. Search → Embedder: Generate query embedding (cache miss)
5. Search → RDS Replica: Vector + BM25 search
6. Search → Reranker: Cross-encoder reranking
7. Search → Client: Return ranked results (<150ms P99)
```

### 4.3 Intelligent Search Flow (Context Engine)

```
1. Client → Gateway: POST /v2/intelligence/search
2. Gateway → Context: Forward with session_id
3. Context → Query Parser: Extract entities, classify intent
4. Context → Query Expander: Add synonyms, session context
5. Context → Search: Multi-modal retrieval (vector + BM25 + graph)
6. Context → Fusion: RRF + Citation propagation
7. Context → Stitcher: Build coherent context window
8. Context → Reasoner: Multi-hop reasoning (if deep mode)
9. Context → LLM: Synthesize answer (if synthesis mode)
10. Context → Session Memory: Update session state
11. Context → Client: Return results + context + synthesis
```

---

## 5. Database Architecture

### 5.1 Write Path

- All writes go to RDS Primary
- Ingestion service, embedding workers write
- Strong consistency for paper/chunk creation

### 5.2 Read Path

- Search service reads from Read Replicas
- Eventual consistency acceptable (lag <1s)
- Connection pool: 50 per replica

### 5.3 Caching Strategy

- **Query Cache**: Redis, 5-minute TTL, keyed by query hash
- **Embedding Cache**: Redis, 1-hour TTL, keyed by text hash
- **Session Cache**: Redis, 30-minute TTL sliding window

### 5.4 Index Strategy

- **HNSW Index**: m=16, ef_construction=64 for vector search
- **GIN Index**: Full-text search on content
- **B-tree Index**: Tenant, status, timestamps
- **GIN Index**: JSONB metadata queries

---

## 6. Resilience Patterns

### 6.1 Circuit Breaker

```rust
// Configuration for embedding service
CircuitBreaker {
    failure_threshold: 5,       // Open after 5 failures
    success_threshold: 3,       // Close after 3 successes
    timeout: Duration::secs(30), // Half-open timeout
}
```

### 6.2 Retry Policy

```rust
// Exponential backoff
RetryPolicy {
    max_retries: 3,
    initial_delay: Duration::millis(100),
    max_delay: Duration::secs(10),
    multiplier: 2.0,
    jitter: 0.1, // 10% jitter
}
```

### 6.3 Rate Limiting

```rust
// Token bucket per tenant
RateLimiter {
    capacity: 100,           // Max burst
    refill_rate: 50,         // 50 tokens/second
    refill_interval: 1s,
}
```

### 6.4 Graceful Shutdown

```rust
// Shutdown sequence
1. Stop accepting new connections
2. Wait for in-flight requests (30s max)
3. Drain SQS messages
4. Close database connections
5. Flush metrics/traces
6. Exit
```

---

## 7. Observability

### 7.1 Metrics (Prometheus)

```
# Request metrics
paperforge_request_total{service, endpoint, status}
paperforge_request_duration_seconds{service, endpoint, quantile}

# Business metrics
paperforge_papers_ingested_total{tenant}
paperforge_chunks_created_total{tenant, model}
paperforge_search_queries_total{tenant, mode}
paperforge_search_results_count{tenant}

# System metrics
paperforge_db_connections_active{pool}
paperforge_queue_depth{queue}
paperforge_cache_hit_ratio{cache}
paperforge_embedding_latency_seconds{provider}
```

### 7.2 Traces (OpenTelemetry)

```
// Span hierarchy
Gateway (entry)
  └── Auth (middleware)
  └── RateLimit (middleware)
  └── Search (service call)
        └── Embed Query (external)
        └── Vector Search (db)
        └── BM25 Search (db)
        └── Rerank (compute)
```

### 7.3 Logs (Structured JSON)

```json
{
  "timestamp": "2026-02-07T19:30:00Z",
  "level": "info",
  "service": "search",
  "trace_id": "abc123",
  "span_id": "def456",
  "message": "Search completed",
  "query_len": 45,
  "results": 20,
  "duration_ms": 87
}
```

---

## 8. Security

### 8.1 Authentication

- API Keys (SHA256 hashed in DB)
- JWT tokens for user sessions
- mTLS between internal services

### 8.2 Authorization

- Tenant isolation at query level
- API key scopes (read, write, admin)
- Row-level security in PostgreSQL

### 8.3 Data Protection

- Encryption at rest (RDS, S3)
- Encryption in transit (TLS 1.3)
- PII masking in logs
- Secrets in AWS Secrets Manager

---

## 9. Deployment

### 9.1 Container Strategy

- Multi-stage Docker builds
- Distroless base images
- Health check endpoints

### 9.2 Kubernetes Resources

- Deployments with rolling updates
- HPA based on CPU/queue depth
- PodDisruptionBudgets
- Network Policies

### 9.3 Infrastructure

- Terraform for AWS resources
- GitHub Actions for CI/CD
- Staging -> Production promotion

---

## 10. Cost Model (Monthly Estimate)

| Resource          | Specification   | Cost            |
| ----------------- | --------------- | --------------- |
| ALB               | Standard        | $18             |
| ECS/EKS (API)     | 3x 0.5vCPU, 1GB | $45             |
| ECS/EKS (Workers) | 2x 1vCPU, 2GB   | $60             |
| RDS PostgreSQL    | db.r6g.large    | $120            |
| RDS Read Replica  | db.r6g.large    | $120            |
| ElastiCache Redis | cache.t3.small  | $25             |
| SQS               | Standard queue  | $1              |
| CloudWatch        | Logs + Metrics  | $15             |
| S3                | Documents       | $5              |
| **Total**         |                 | **~$410/month** |

---

## Appendix: ADRs

### ADR-001: Microservice Split

**Decision**: Decompose into 4 services (Gateway, Ingestion, Search, Context)
**Rationale**: Independent scaling, failure isolation, team ownership

### ADR-002: Async Ingestion

**Decision**: Use SQS for embedding job queue
**Rationale**: Decouple API latency from embedding time, enable retries

### ADR-003: Hybrid Search Default

**Decision**: RRF fusion of vector + BM25 as default search mode
**Rationale**: Better recall than pure vector, handles keyword queries

### ADR-004: Session-Based Intelligence

**Decision**: Track user sessions for query context
**Rationale**: Enable "find more like this", query refinement

### ADR-005: Multi-Provider Embeddings

**Decision**: Abstract embedding generation behind interface
**Rationale**: Provider flexibility, cost optimization, local fallback
