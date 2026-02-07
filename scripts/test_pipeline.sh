#!/bin/bash
# PaperForge Pipeline Test Script
# Tests the full ingestion -> embedding -> search pipeline locally

set -e

echo "======================================"
echo "PaperForge Pipeline Test"
echo "======================================"

# Check for Docker
if ! command -v docker &> /dev/null; then
    echo "ERROR: Docker is required. Please install Docker."
    exit 1
fi

# Check for PDFs
PDF_DIR="./data/raw"
if [ ! -d "$PDF_DIR" ] || [ -z "$(ls -A $PDF_DIR/*.pdf 2>/dev/null)" ]; then
    echo "WARNING: No PDFs found in $PDF_DIR"
    echo "Run: python scripts/fetch_arxiv.py"
    exit 1
fi

PDF_COUNT=$(ls -1 $PDF_DIR/*.pdf 2>/dev/null | wc -l)
echo "Found $PDF_COUNT PDFs in $PDF_DIR"

# Step 1: Start infrastructure
echo ""
echo "Step 1: Starting infrastructure..."
docker-compose -f docker-compose.local.yml up -d

# Wait for services to be ready
echo "Waiting for PostgreSQL..."
for i in {1..30}; do
    if docker exec paperforge-postgres pg_isready -U paperforge &>/dev/null; then
        echo "PostgreSQL is ready!"
        break
    fi
    sleep 1
done

echo "Waiting for Redis..."
for i in {1..10}; do
    if docker exec paperforge-redis redis-cli ping &>/dev/null; then
        echo "Redis is ready!"
        break
    fi
    sleep 1
done

echo "Waiting for LocalStack..."
for i in {1..20}; do
    if curl -s http://localhost:4566/_localstack/health | grep -q "running"; then
        echo "LocalStack is ready!"
        break
    fi
    sleep 1
done

# Step 2: Build the services
echo ""
echo "Step 2: Building Rust services..."
cargo build --release --package paperforge-ingestion --package paperforge-embedding-worker

# Step 3: Run ingestion on sample PDF
echo ""
echo "Step 3: Running ingestion..."

# Pick first PDF
FIRST_PDF=$(ls $PDF_DIR/*.pdf | head -1)
echo "Processing: $FIRST_PDF"

# Copy .env.local to .env if not exists
if [ ! -f ".env" ]; then
    cp .env.local .env
fi

# Run ingestion
./target/release/ingestion process-file "$FIRST_PDF"

# Step 4: Test embedding (mock mode)
echo ""
echo "Step 4: Testing embedding service..."
./target/release/embedding-worker test "This is a test sentence for embedding generation."

echo ""
echo "======================================"
echo "Pipeline test complete!"
echo "======================================"
echo ""
echo "Next steps:"
echo "1. Process more PDFs: ./target/release/ingestion process-dir ./data/raw"
echo "2. Start gateway: cargo run --package paperforge-gateway"
echo "3. Query API: curl http://localhost:3000/health"
