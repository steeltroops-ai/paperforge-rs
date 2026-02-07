# PaperForge-rs: Deployment Guide

**Version**: 2.0  
**Status**: Production  
**Last Updated**: 2026-02-07

---

## 1. Prerequisites

### 1.1 Local Development

```bash
# Required tools
- Rust 1.75+ (rustup)
- Docker & Docker Compose
- PostgreSQL 15+ with pgvector extension
- Redis 7+
- bun (for any frontend tooling)

# AWS CLI (for deployment)
- aws-cli v2
- kubectl
- terraform 1.5+
```

### 1.2 AWS Resources

- VPC with public/private subnets
- RDS PostgreSQL with pgvector
- ElastiCache Redis
- SQS queues
- ECR repository
- ECS/EKS cluster
- ALB
- Secrets Manager
- CloudWatch

---

## 2. Local Development Setup

### 2.1 Clone and Build

```bash
git clone https://github.com/your-org/paperforge-rs.git
cd paperforge-rs

# Build all services
cargo build --workspace

# Run tests
cargo test --workspace
```

### 2.2 Docker Compose (Full Stack)

```bash
# Start all dependencies
docker-compose up -d

# Services available:
# - PostgreSQL: localhost:5432
# - Redis: localhost:6379
# - Prometheus: localhost:9090
# - Grafana: localhost:3000
```

### 2.3 Run Individual Services

```bash
# Set environment
export DATABASE_URL="postgres://paperforge:paperforge@localhost:5432/paperforge"
export REDIS_URL="redis://localhost:6379"
export EMBEDDING_API_KEY="your-openai-key"

# Run gateway
cargo run -p gateway

# Run search service
cargo run -p search

# Run ingestion service
cargo run -p ingestion

# Run embedding worker
cargo run -p embedding-worker
```

---

## 3. Configuration

### 3.1 Environment Variables

| Variable              | Required | Default      | Description                      |
| --------------------- | -------- | ------------ | -------------------------------- |
| `DATABASE_URL`        | Yes      | -            | PostgreSQL connection string     |
| `DATABASE_READ_URL`   | No       | DATABASE_URL | Read replica connection          |
| `REDIS_URL`           | Yes      | -            | Redis connection string          |
| `EMBEDDING_API_KEY`   | Yes      | -            | OpenAI/Anthropic API key         |
| `EMBEDDING_PROVIDER`  | No       | `openai`     | `openai`, `anthropic`, `local`   |
| `SQS_INGESTION_QUEUE` | Prod     | -            | SQS queue URL                    |
| `AWS_REGION`          | Prod     | `us-east-1`  | AWS region                       |
| `LOG_LEVEL`           | No       | `info`       | `debug`, `info`, `warn`, `error` |
| `OTEL_ENDPOINT`       | No       | -            | OpenTelemetry collector          |
| `PORT`                | No       | `8080`       | Service port                     |

### 3.2 Secrets Management

```bash
# Store secrets in AWS Secrets Manager
aws secretsmanager create-secret \
  --name paperforge/prod/database \
  --secret-string '{"url":"postgres://..."}'

aws secretsmanager create-secret \
  --name paperforge/prod/embedding \
  --secret-string '{"api_key":"sk-..."}'
```

---

## 4. Database Setup

### 4.1 Create Database

```bash
# Connect to PostgreSQL
psql -h localhost -U postgres

# Create database and user
CREATE DATABASE paperforge;
CREATE USER paperforge WITH PASSWORD 'your-secure-password';
GRANT ALL PRIVILEGES ON DATABASE paperforge TO paperforge;
```

### 4.2 Run Migrations

```bash
# Apply schema
psql -h localhost -U paperforge -d paperforge -f docs/schema.sql

# Verify extensions
psql -c "SELECT * FROM pg_extension WHERE extname = 'vector';"
```

### 4.3 Create Read Replica (Production)

```bash
# AWS CLI
aws rds create-db-instance-read-replica \
  --db-instance-identifier paperforge-read-1 \
  --source-db-instance-identifier paperforge-primary \
  --db-instance-class db.r6g.large
```

---

## 5. Docker Build

### 5.1 Multi-Stage Dockerfile

```dockerfile
# Builder stage
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --workspace

# Runtime stage
FROM gcr.io/distroless/cc-debian12
COPY --from=builder /app/target/release/gateway /app/gateway
COPY --from=builder /app/target/release/search /app/search
COPY --from=builder /app/target/release/ingestion /app/ingestion
COPY --from=builder /app/target/release/embedding-worker /app/embedding-worker

# Default to gateway
ENTRYPOINT ["/app/gateway"]
```

### 5.2 Build and Push

```bash
# Build
docker build -t paperforge-rs:latest .

# Tag for ECR
aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin 123456789.dkr.ecr.us-east-1.amazonaws.com

docker tag paperforge-rs:latest 123456789.dkr.ecr.us-east-1.amazonaws.com/paperforge:latest
docker push 123456789.dkr.ecr.us-east-1.amazonaws.com/paperforge:latest
```

---

## 6. Kubernetes Deployment

### 6.1 Namespace and Secrets

```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: paperforge

---
# secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: paperforge-secrets
  namespace: paperforge
type: Opaque
stringData:
  DATABASE_URL: "postgres://..."
  REDIS_URL: "redis://..."
  EMBEDDING_API_KEY: "sk-..."
```

### 6.2 Deployment (Gateway)

```yaml
# gateway-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: gateway
  namespace: paperforge
spec:
  replicas: 3
  selector:
    matchLabels:
      app: gateway
  template:
    metadata:
      labels:
        app: gateway
    spec:
      containers:
        - name: gateway
          image: 123456789.dkr.ecr.us-east-1.amazonaws.com/paperforge:latest
          command: ["/app/gateway"]
          ports:
            - containerPort: 8080
          envFrom:
            - secretRef:
                name: paperforge-secrets
          resources:
            requests:
              cpu: "250m"
              memory: "512Mi"
            limits:
              cpu: "500m"
              memory: "1Gi"
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /ready
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 5
```

### 6.3 Horizontal Pod Autoscaler

```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: gateway-hpa
  namespace: paperforge
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: gateway
  minReplicas: 3
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

### 6.4 Service and Ingress

```yaml
# service.yaml
apiVersion: v1
kind: Service
metadata:
  name: gateway
  namespace: paperforge
spec:
  selector:
    app: gateway
  ports:
    - port: 80
      targetPort: 8080

---
# ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: paperforge-ingress
  namespace: paperforge
  annotations:
    kubernetes.io/ingress.class: alb
    alb.ingress.kubernetes.io/scheme: internet-facing
spec:
  rules:
    - host: api.paperforge.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: gateway
                port:
                  number: 80
```

---

## 7. CI/CD Pipeline

### 7.1 GitHub Actions

```yaml
# .github/workflows/deploy.yml
name: Deploy

on:
  push:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: pgvector/pgvector:pg15
        env:
          POSTGRES_PASSWORD: test
        ports:
          - 5432:5432
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test --workspace

  build:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: docker/setup-buildx-action@v3
      - uses: aws-actions/configure-aws-credentials@v4
        with:
          aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
          aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          aws-region: us-east-1
      - uses: aws-actions/amazon-ecr-login@v2
      - run: |
          docker build -t $ECR_REGISTRY/paperforge:${{ github.sha }} .
          docker push $ECR_REGISTRY/paperforge:${{ github.sha }}

  deploy-staging:
    needs: build
    runs-on: ubuntu-latest
    environment: staging
    steps:
      - uses: actions/checkout@v4
      - run: |
          kubectl set image deployment/gateway gateway=$ECR_REGISTRY/paperforge:${{ github.sha }}
          kubectl rollout status deployment/gateway

  deploy-production:
    needs: deploy-staging
    runs-on: ubuntu-latest
    environment: production
    steps:
      - uses: actions/checkout@v4
      - run: |
          kubectl set image deployment/gateway gateway=$ECR_REGISTRY/paperforge:${{ github.sha }}
          kubectl rollout status deployment/gateway
```

---

## 8. Monitoring Setup

### 8.1 Prometheus ServiceMonitor

```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: paperforge
  namespace: paperforge
spec:
  selector:
    matchLabels:
      app: gateway
  endpoints:
    - port: http
      path: /metrics
      interval: 15s
```

### 8.2 Grafana Dashboard

Import the dashboard JSON from `deploy/grafana/paperforge-dashboard.json`.

Key panels:

- Request rate by endpoint
- P50/P90/P99 latency
- Error rate by type
- Database connection pool
- Queue depth
- Cache hit ratio

---

## 9. Runbook

### 9.1 Health Checks

```bash
# Check liveness
curl http://localhost:8080/health

# Check readiness (includes dependencies)
curl http://localhost:8080/ready
```

### 9.2 Common Issues

| Symptom              | Likely Cause              | Resolution                 |
| -------------------- | ------------------------- | -------------------------- |
| 503 on /ready        | Database down             | Check RDS status           |
| High latency         | Connection pool exhausted | Scale up, increase pool    |
| Queue backup         | Workers crashed           | Check worker logs, restart |
| Empty search results | Index corruption          | Re-index affected chunks   |

### 9.3 Rollback

```bash
# Kubernetes
kubectl rollout undo deployment/gateway

# ECS
aws ecs update-service --cluster paperforge --service gateway --task-definition paperforge:previous
```

---

## 10. Disaster Recovery

### 10.1 Backups

- RDS: Automated daily snapshots, 7-day retention
- Point-in-time recovery enabled
- S3: Cross-region replication for documents

### 10.2 Recovery Procedure

```bash
# Restore from snapshot
aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier paperforge-restored \
  --db-snapshot-identifier paperforge-snapshot-2026-02-07
```

### 10.3 RTO/RPO

- **RTO** (Recovery Time Objective): 1 hour
- **RPO** (Recovery Point Objective): 5 minutes (point-in-time recovery)
