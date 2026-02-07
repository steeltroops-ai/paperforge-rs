# PaperForge-rs

## Production-grade MVP Async Rust Microservice for Semantic Research Paper Retrieval

**Status**: Active Development
**Technology**: Rust, Axum, PostgreSQL, pgvector, Docker

### Overview

PaperForge-rs is a high-performance backend microservice designed for indexing, embedding, and semantic retrieval of academic research papers. It leverages the power of async Rust, `pgvector`, and modern observability practices to provide a scalable and robust solution for research discovery.

This project demonstrates systems-level thinking in building cloud-native applications, prioritizing correctness, performance, and maintainability.

### Key Features

- **Semantic Search**: Utilizes vector embeddings (768d) for context-aware search, moving beyond simple keyword matching.
- **Hybrid Retrieval**: Combines vector similarity with structured metadata filtering (e.g., date, source).
- **High Performance**: Built on the Tokio runtime and Axum web framework for lightning-fast concurrent request handling.
- **Production Ready**: Includes comprehensive error handling, structured logging (tracing), and Prometheus metrics.
- **Scalable Architecture**: Stateless API design, connection pooling, and HNSW indexing for handling millions of vectors.
- **Containerized**: Fully Dockerized with Docker Compose for easy local development and cloud deployment.

### Tech Stack

- **Language**: Rust (2021 Edition)
- **Web Framework**: Axum
- **Database**: PostgreSQL 16+ with pgvector
- **ORM**: Sea-ORM
- **Serialization**: Serde
- **Error Handling**: `thiserror` (library), `anyhow` (application)
- **Observability**: `tracing`, `tracing-subscriber`, `opentelemetry`
- **Metrics**: Prometheus exporter via `axum-prometheus`
- **Configuration**: `dotenv` + Typed Config Struct
- **Embeddings**: Interface compatible with OpenAI API or local inference servers.

### Architecture Structure

```text
src/
├── routes/        # HTTP handlers and input validation
├── services/      # Business logic (Ingestion, Search, Embedding)
├── db/            # Database access layer and repositories
├── embeddings/    # Embedding provider integration
├── metrics/       # Telemetry configuration
├── config.rs      # Type-safe configuration
└── main.rs        # Application entry point
```

### Getting Started

#### Prerequisites

- Rust (Latest Stable)
- Docker & Docker Compose
- PostgreSQL client (psql, optional)

#### Local Development

1.  **Clone the repository:**

    ```bash
    git clone https://github.com/yourusername/paperforge-rs.git
    cd paperforge-rs
    ```

2.  **Start Infrastructure:**
    Start the database and monitoring services in the background.

    ```bash
    docker-compose up -d db prometheus
    ```

3.  **Run Migrations:**
    Ensure the database schema is applied.

    ```bash
    # Install SeaORM CLI if you haven't already
    cargo install sea-orm-cli

    # Run migrations (assuming migration scripts exist or using schema.sql manually)
    # For MVP, you can apply the schema.sql:
    docker-compose exec -T db psql -U postgres -d paperforge < docs/schema.sql
    ```

4.  **Run Application:**
    ```bash
    cargo run
    ```
    The API will listen on `0.0.0.0:3000`.

#### Testing

Run the comprehensive test suite:

```bash
cargo test
```

### Observability

- **Metrics**: `GET /metrics` produces Prometheus-formatted metrics.
- **Health**: `GET /health` provides a liveness probe.
- **Logs**: Structured JSON logs are emitted to stdout for aggregation (e.g., via FluentBit/Jaeger).

### Deployment Strategy (AWS)

1.  **Container Registry**: Push Docker image to ECR.
2.  **Compute**: Deploy to ECS Fargate behind an Application Load Balancer.
3.  **Database**: Provision RDS PostgreSQL (v16+) with the `vector` extension enabled.
4.  **Configuration**: Inject `APP_DATABASE__URL` and keys via AWS Secrets Manager.

### License

MIT License.
