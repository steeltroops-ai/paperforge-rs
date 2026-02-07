-- =========================================================================================
-- Partitioning Strategy for 'chunks' Table
-- Hash Partitioning by 'paper_id' (16 partitions)
-- =========================================================================================

BEGIN;

-- 1. Create the new partitioned table
CREATE TABLE chunks_partitioned (
    id UUID NOT NULL,
    paper_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    chunk_index INT NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    embedding VECTOR(1536),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id, paper_id) -- Partition key must be part of PK
) PARTITION BY HASH (paper_id);

-- 2. Create partitions (0-15)
CREATE TABLE chunks_p0 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 0);
CREATE TABLE chunks_p1 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 1);
CREATE TABLE chunks_p2 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 2);
CREATE TABLE chunks_p3 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 3);
CREATE TABLE chunks_p4 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 4);
CREATE TABLE chunks_p5 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 5);
CREATE TABLE chunks_p6 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 6);
CREATE TABLE chunks_p7 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 7);
CREATE TABLE chunks_p8 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 8);
CREATE TABLE chunks_p9 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 9);
CREATE TABLE chunks_p10 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 10);
CREATE TABLE chunks_p11 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 11);
CREATE TABLE chunks_p12 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 12);
CREATE TABLE chunks_p13 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 13);
CREATE TABLE chunks_p14 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 14);
CREATE TABLE chunks_p15 PARTITION OF chunks_partitioned FOR VALUES WITH (MODULUS 16, REMAINDER 15);

-- 3. Copy data (if existing table is not empty)
INSERT INTO chunks_partitioned
SELECT * FROM chunks;

-- 4. Swap tables safely
ALTER TABLE chunks RENAME TO chunks_old;
ALTER TABLE chunks_partitioned RENAME TO chunks;

-- 5. Create Indexes on Partitioned Table (Postgres 11+ propagates to partitions)
CREATE INDEX idx_chunks_content_bm25 ON chunks USING GIN (to_tsvector('english', content));
CREATE INDEX idx_chunks_paper_id ON chunks (paper_id);
-- Note: Vector indexes (HNSW) should be created on partitions individually for performance, 
-- or use ivfflat on partitioned table (requires careful maintenance).
-- For now, create HNSW on each partition manually or rely on propagation if supported by pgvector version.

-- 6. Cleanup (Optional: verify data before dropping)
-- DROP TABLE chunks_old;

COMMIT;
