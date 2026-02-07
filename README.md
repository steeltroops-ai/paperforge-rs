# PaperForge-rs

**Production-grade Async Rust Microservice for Semantic Research Paper Retrieval**

![Stack](https://img.shields.io/badge/ech-Rust%20%7C%20Axum%20%7C%20Postgres-orange)

## Overview

High-performance backend for indexing and semantic retrieval of academic papers. Implements "Hybrid Search" (Vector Cosine Similarity + Full-Text Rank) using pgvector and HNSW indices.

## Tech Stack

- **Core**: Rust 2021 (Tokio, Axum)
- **Data**: PostgreSQL 16 (`pgvector`, `uuid-ossp`)
- **ORM**: Sea-ORM + SQLx (Raw SQL for vector ops)
- **Ops**: Docker Compose, Prometheus Metrics, Tracing

## Project Structure

Hexagonal architecture decoupling domain logic from infrastructure.

```text
crates/
├── common/            # Shared Types, Config, Errors
├── gateway/           # API Gateway (Auth, Rate Limiting, Routing)
├── search/            # Search Service (Vector, BM25, Hybrid)
├── ingestion/         # Ingestion Service (Async Processing)
├── context/           # Context Management
└── embedding-worker/  # Embedding Generation Worker
```

## Getting Started

### Prerequisites

- Docker & Docker Compose
- Rust Toolchain (Latest Stable)

### Local Development

1.  **Setup Configuration**:

    ```bash
    cp .env.example .env
    # Edit .env with your local settings
    ```

2.  **Start Infrastructure**:

    ```bash
    docker-compose up -d db prometheus
    ```

3.  **Apply Schema**:

    ```bash
    cargo install sea-orm-cli
    docker-compose exec -T db psql -U postgres -d paperforge < docs/schema.sql
    ```

4.  **Run Services**:

    ```bash
    # Run Gateway (API)
    cargo run -p paperforge-gateway
    # Listening on http://0.0.0.0:3000

    # Run other services (in separate terminals as needed)
    cargo run -p paperforge-search
    cargo run -p paperforge-ingestion
    cargo run -p paperforge-context
    cargo run -p paperforge-embedding-worker
    ```

## API Usage

**Ingest Paper**

```bash
curl -X POST http://localhost:3000/ingest \
  -H "Content-Type: application/json" \
  -d '{"title":"Rust Systems","abstract_text":"Memory safety without GC..."}'
```

**Hybrid Search**

```bash
curl "http://localhost:3000/search?q=memory+safety&hybrid=true"
```

## Observability

- **Metrics**: `GET /metrics` (Prometheus)
- **Health**: `GET /health`
- **Logs**: JSON structured logging to stdout.

## Deployment

Designed for **AWS ECS Fargate** + **RDS PostgreSQL**.

1.  Push Docker image.
2.  Provision RDS with `vector` extension.
3.  Inject config via Environment Variables (`APP__DATABASE__URL`, `APP__EMBEDDING__API_KEY`).
