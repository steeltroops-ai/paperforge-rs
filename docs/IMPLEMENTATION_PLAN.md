# PaperForge-rs V2: Master Implementation Plan

**Version**: 2.0  
**Date**: 2026-02-07  
**Status**: Active Development Plan  
**Architecture**: Microservice Platform

---

## 1. Executive Overview

This document consolidates all V2 requirements, architecture decisions, and implementation roadmap into a single source of truth. It supersedes all V1 documentation and aligns with the Microsoft-style architecture patterns for production-grade research infrastructure.

### Target Architecture: 4-Service Decomposition

```
paperforge-platform/
├── services/
│   ├── api-gateway/          # Auth, rate limiting, routing
│   ├── ingestion-service/    # Async paper processing, chunking
│   ├── search-service/       # Vector + hybrid search, ranking
│   └── context-engine/       # Intelligence layer, LLM integration
├── workers/
│   └── embedding-worker/     # Background embedding generation
├── shared/
│   ├── proto/                # gRPC service definitions
│   └── common/               # Shared Rust library
└── infrastructure/
    └── terraform/            # AWS IaC
```

---

## 2. Phase Breakdown

### Phase 1: Production Foundation (Week 1-2)

**Goal**: Make current codebase production-ready

| Task                                 | Priority | Status      |
| ------------------------------------ | -------- | ----------- |
| P1.1: Fix compilation (MSVC)         | CRITICAL | Pending     |
| P1.2: Circuit breaker on embeddings  | CRITICAL | **DONE**    |
| P1.3: Async ingestion with job queue | HIGH     | **DONE**    |
| P1.4: Rate limiting (token bucket)   | HIGH     | **DONE**    |
| P1.5: OpenTelemetry integration      | HIGH     | In Progress |
| P1.6: Test coverage >60%             | HIGH     | Not Started |
| P1.7: Kubernetes manifests           | MEDIUM   | Not Started |

### Phase 2: Microservice Split (Week 3-4)

**Goal**: Decompose monolith into services

| Task                                   | Priority | Status          |
| -------------------------------------- | -------- | --------------- |
| P2.1: Extract API Gateway service      | HIGH     | **DONE**        |
| P2.2: Extract Ingestion Service        | HIGH     | **DONE** (stub) |
| P2.3: Extract Search Service           | HIGH     | **DONE** (stub) |
| P2.4: gRPC inter-service communication | HIGH     | **DONE**        |
| P2.5: SQS queue integration            | HIGH     | **DONE**        |
| P2.6: Redis caching layer              | MEDIUM   | **DONE**        |

### Phase 3: Context Engine (Week 5-8)

**Goal**: Implement Augment-like intelligence layer

| Task                                                | Priority | Status   |
| --------------------------------------------------- | -------- | -------- |
| P3.1: Query Parser + Intent Classification          | HIGH     | **DONE** |
| P3.2: Query Expander (synonyms, session)            | HIGH     | **DONE** |
| P3.3: Multi-modal retrieval (vector + BM25 + graph) | HIGH     | **DONE** |
| P3.4: Context Stitcher                              | HIGH     | **DONE** |
| P3.5: Session Memory (Redis)                        | MEDIUM   | **DONE** |
| P3.6: Multi-Hop Reasoner                            | MEDIUM   | **DONE** |
| P3.7: LLM Integration Layer                         | LOW      | **DONE** |
| P3.8: Citation Propagation Scoring                  | LOW      | **DONE** |

### Phase 4: Scale & Reliability (Week 9-12)

**Goal**: 10M+ paper capacity

| Task                              | Priority | Status      |
| --------------------------------- | -------- | ----------- |
| P4.1: Read/Write DB separation    | HIGH     | **DONE**    |
| P4.2: Table partitioning strategy | MEDIUM   | Not Started |
| P4.3: Multi-tenant isolation      | MEDIUM   | **DONE**    |
| P4.4: Dead letter queue           | MEDIUM   | **DONE**    |
| P4.5: Auto-scaling policies       | LOW      | Not Started |

---

## 3. New Project Structure (V2) - IMPLEMENTED

```text
paperforge-rs/
├── Cargo.toml                    # Workspace root (workspace members defined)
├── crates/
│   ├── common/                   # Shared library (paperforge-common)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── auth/             # JWT, API key validation
│   │       ├── config/           # Configuration management
│   │       ├── db/               # Database models, repository
│   │       │   ├── models/       # SeaORM entities
│   │       │   └── repository.rs # Data access layer
│   │       ├── embeddings/       # Embedder abstraction
│   │       ├── errors/           # Error types, HTTP mapping
│   │       └── metrics/          # Prometheus metrics
│   ├── gateway/                  # API Gateway (paperforge-gateway)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── handlers/         # HTTP route handlers
│   │       │   ├── health.rs
│   │       │   ├── papers.rs
│   │       │   ├── jobs.rs
│   │       │   ├── search.rs
│   │       │   ├── intelligence.rs
│   │       │   ├── sessions.rs
│   │       │   └── citations.rs
│   │       └── middleware/       # Rate limiting, auth
│   ├── ingestion/                # Ingestion Service (stub)
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   ├── search/                   # Search Service (stub)
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   ├── context/                  # Context Engine (stub)
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   └── embedding-worker/         # Embedding Worker (stub)
│       ├── Cargo.toml
│       └── src/main.rs
├── docs/
│   ├── ARCHITECTURE.md           # System architecture
│   ├── API_REFERENCE.md          # API documentation
│   ├── CONTEXT_ENGINE_DESIGN.md  # Intelligence layer design
│   ├── DEPLOYMENT.md             # Deployment guide
│   ├── IMPLEMENTATION_PLAN.md    # This document
│   ├── PRD.md                    # Product requirements
│   └── schema.sql                # V2 database schema
├── deploy/                       # [TODO] Kubernetes, Terraform
└── proto/                        # [TODO] gRPC definitions
```

---

## 4. Database Schema V2

The V2 schema includes:

1. **Multi-tenant support** - `tenants` table with isolated data
2. **Embedding versioning** - Track model version per chunk
3. **Citation graph** - `citations` table for graph queries
4. **Job tracking** - `ingestion_jobs` for async processing
5. **Metadata extensibility** - JSONB for flexible attributes
6. **Partitioning ready** - By embedding model for 10M+ scale

See full schema in `docs/SCHEMA.md`.

---

## 5. API Contracts V2

### Authentication

All endpoints require:

```http
Authorization: Bearer <api_key>
X-Tenant-ID: <tenant_uuid>
X-Request-ID: <correlation_id>
```

### Endpoints

| Method | Path                      | Service   | Description                       |
| ------ | ------------------------- | --------- | --------------------------------- |
| POST   | /v2/papers                | Ingestion | Async paper ingestion             |
| GET    | /v2/jobs/{id}             | Ingestion | Job status                        |
| POST   | /v2/search                | Search    | Hybrid search                     |
| POST   | /v2/intelligence/search   | Context   | Intelligent search with reasoning |
| POST   | /v2/sessions              | Context   | Create session                    |
| GET    | /v2/papers/{id}/citations | Search    | Citation graph                    |
| GET    | /health                   | All       | Liveness probe                    |
| GET    | /ready                    | All       | Readiness probe                   |

---

## 6. Context Engine Components

### 6.1 Query Understanding Pipeline

```
User Query -> Query Parser -> Query Expander -> Query Graph Builder
                │                  │                    │
                v                  v                    v
        Extract Entities    Add Synonyms        Link to Papers
        Classify Intent     Session Context     Build Relations
```

### 6.2 Retrieval Fusion

```
Query Graph -> [Vector Search, BM25 Search, Graph Search, Temporal Search]
                                       │
                                       v
                            Reciprocal Rank Fusion
                                       │
                                       v
                        Citation Propagation Scoring
                                       │
                                       v
                            Context Stitcher
```

### 6.3 Intelligence Layer

```
Stitched Context -> Reranker -> Multi-Hop Reasoner -> LLM Synthesizer
                                       │                     │
                                       v                     v
                             Session Memory          Synthesized Answer
```

---

## 7. Service Communication

### Internal (gRPC)

- Search <-> Context: Query rewriting, context fetching
- Ingestion <-> Search: Re-indexing triggers
- All -> Embedding Worker: Embedding requests via SQS

### External (REST/HTTP)

- Gateway -> All services: Authenticated requests
- All services -> Prometheus: Metrics scrape
- All services -> OTEL Collector: Traces

---

## 8. Observability Stack

| Component     | Purpose               | Target       |
| ------------- | --------------------- | ------------ |
| Prometheus    | Metrics collection    | All services |
| Grafana       | Dashboards & alerting | Prometheus   |
| OpenTelemetry | Distributed tracing   | All services |
| CloudWatch    | AWS native logging    | All services |
| Loki          | Log aggregation       | All services |

### Key SLOs

- Search P99 < 150ms
- Ingestion ack < 100ms
- Availability 99.9%
- Error rate < 0.1%

---

## 9. Immediate Next Steps

### Today's Focus (Priority Order)

1. **Clean up docs folder** - Delete obsolete files, keep only V2 docs
2. **Create workspace structure** - Setup Cargo workspace for multi-service
3. **Extract common crate** - Move shared code to `common/`
4. **Implement circuit breaker** - Add to embedding client
5. **Add async job queue** - Implement ingestion jobs with status

---

## 10. Documentation Consolidation

### Files to DELETE (Obsolete V1)

- `PRD.md` (superseded by PRD_V2.md)
- `ARCHITECTURE_REVIEW.md` (merged into this plan)
- `SYSTEM_ARCHITECTURE.md` (merged into ARCHITECTURE.md)
- `PRODUCTION_CHECKLIST.md` (merged into this plan)
- `DEPLOYMENT_ARCHITECTURE.md` (merged into DEPLOYMENT.md)
- `ROADMAP.md` (superseded by this plan)
- `API_CONTRACT.md` (superseded by API_REFERENCE.md)
- `PRODUCTION_AUDIT_REPORT.md` (historical, archive)
- `conext.md` (typo, merged into CONTEXT_ENGINE_DESIGN.md)

### Files to KEEP/UPDATE

- `PRD_V2.md` -> Rename to `PRD.md`
- `CONTEXT_ENGINE_DESIGN.md` -> Keep as reference
- `schema.sql` -> Update to V2 schema

### Files to CREATE

- `ARCHITECTURE.md` - Consolidated architecture doc
- `API_REFERENCE.md` - OpenAPI spec
- `DEPLOYMENT.md` - AWS deployment guide
- `RUNBOOK.md` - Operations runbook

---

## Appendix A: Technology Stack

| Layer         | Technology            | Justification           |
| ------------- | --------------------- | ----------------------- |
| Language      | Rust                  | Performance, safety     |
| Web Framework | Axum                  | Async, tower middleware |
| Database      | PostgreSQL + pgvector | Vector search, JSONB    |
| ORM           | SeaORM                | Async, type-safe        |
| Cache         | Redis                 | Session, query cache    |
| Queue         | AWS SQS               | Managed, reliable       |
| Container     | Docker                | Portability             |
| Orchestration | Kubernetes            | Scalability             |
| CI/CD         | GitHub Actions        | Integration             |
| Observability | Prometheus + OTEL     | Industry standard       |

---

**Document Status**: Living document - update as implementation progresses
