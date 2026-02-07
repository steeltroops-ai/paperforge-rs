-- PaperForge-rs Database Schema V2
-- Production-grade schema with:
-- - Multi-tenant support
-- - Embedding versioning
-- - Citation graph
-- - Job tracking
-- - Full-text search
-- - Partitioning ready

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "vector";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- =========================================================================
-- TENANTS TABLE (Multi-tenant support)
-- =========================================================================
CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    api_key_hash TEXT NOT NULL,
    rate_limit_rps INT DEFAULT 100,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tenants_api_key ON tenants(api_key_hash) WHERE is_active = true;

-- =========================================================================
-- EMBEDDING MODELS REGISTRY
-- =========================================================================
CREATE TABLE IF NOT EXISTS embedding_models (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    provider TEXT NOT NULL,
    dimension INT NOT NULL,
    is_active BOOLEAN DEFAULT true,
    is_default BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
);

-- Insert default model
INSERT INTO embedding_models (name, provider, dimension, is_default) 
VALUES ('text-embedding-ada-002', 'openai', 768, true)
ON CONFLICT (name) DO NOTHING;

-- =========================================================================
-- PAPERS TABLE
-- =========================================================================
CREATE TABLE IF NOT EXISTS papers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    external_id TEXT,  -- DOI, ArXiv ID, etc.
    title TEXT NOT NULL,
    abstract_text TEXT NOT NULL,
    published_at TIMESTAMPTZ,
    source TEXT,
    
    -- Extensible metadata as JSONB
    -- Example: {"doi": "10.1234/...", "authors": [...], "keywords": [...]}
    metadata JSONB DEFAULT '{}' NOT NULL,
    
    -- Idempotency key for deduplication (SHA256 hash or client-provided)
    idempotency_key TEXT,
    
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    
    CONSTRAINT papers_tenant_external_unique UNIQUE(tenant_id, external_id),
    CONSTRAINT papers_tenant_idempotency_unique UNIQUE(tenant_id, idempotency_key)
);

-- Indexes for papers
CREATE INDEX IF NOT EXISTS idx_papers_tenant ON papers(tenant_id);
CREATE INDEX IF NOT EXISTS idx_papers_external ON papers(tenant_id, external_id);
CREATE INDEX IF NOT EXISTS idx_papers_published ON papers(published_at);
CREATE INDEX IF NOT EXISTS idx_papers_created ON papers(created_at);
CREATE INDEX IF NOT EXISTS idx_papers_source ON papers(source);
CREATE INDEX IF NOT EXISTS idx_papers_metadata ON papers USING GIN(metadata);

-- Full-text search on title
CREATE INDEX IF NOT EXISTS idx_papers_title_fts ON papers USING GIN(to_tsvector('english', title));

-- =========================================================================
-- CHUNKS TABLE
-- =========================================================================
CREATE TABLE IF NOT EXISTS chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    paper_id UUID NOT NULL REFERENCES papers(id) ON DELETE CASCADE,
    chunk_index INT NOT NULL,
    content TEXT NOT NULL,
    
    -- Vector embedding (dimension varies by model)
    embedding vector(768),
    
    -- Embedding versioning for model upgrades
    embedding_model TEXT NOT NULL DEFAULT 'text-embedding-ada-002',
    embedding_version INT NOT NULL DEFAULT 1,
    
    -- Token count for context management
    token_count INT DEFAULT 0 NOT NULL,
    
    -- Character offsets for source mapping
    char_offset_start INT,
    char_offset_end INT,
    
    -- Generated full-text search vector
    text_search_vector tsvector GENERATED ALWAYS AS (to_tsvector('english', content)) STORED,
    
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    
    CONSTRAINT chunks_paper_index_unique UNIQUE(paper_id, chunk_index)
);

-- Indexes for chunks
CREATE INDEX IF NOT EXISTS idx_chunks_paper ON chunks(paper_id);
CREATE INDEX IF NOT EXISTS idx_chunks_model_version ON chunks(embedding_model, embedding_version);
CREATE INDEX IF NOT EXISTS idx_chunks_created ON chunks(created_at);

-- Vector similarity search index (HNSW for better performance)
-- m = number of bidirectional links (higher = better recall, more memory)
-- ef_construction = search depth during build (higher = better recall, slower build)
CREATE INDEX IF NOT EXISTS idx_chunks_embedding_hnsw ON chunks 
USING hnsw (embedding vector_cosine_ops) 
WITH (m = 16, ef_construction = 64);

-- Full-text search index
CREATE INDEX IF NOT EXISTS idx_chunks_content_fts ON chunks USING GIN(text_search_vector);

-- Trigram index for fuzzy matching
CREATE INDEX IF NOT EXISTS idx_chunks_content_trgm ON chunks USING GIN(content gin_trgm_ops);

-- =========================================================================
-- CITATIONS TABLE (Graph)
-- =========================================================================
CREATE TABLE IF NOT EXISTS citations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    citing_paper_id UUID NOT NULL REFERENCES papers(id) ON DELETE CASCADE,
    cited_paper_id UUID NOT NULL REFERENCES papers(id) ON DELETE CASCADE,
    citation_context TEXT,  -- The sentence containing the citation
    position_in_paper INT,  -- Order of citation in paper
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    
    CONSTRAINT citations_unique UNIQUE(citing_paper_id, cited_paper_id),
    CONSTRAINT citations_no_self CHECK(citing_paper_id != cited_paper_id)
);

-- Indexes for citations (bidirectional traversal)
CREATE INDEX IF NOT EXISTS idx_citations_citing ON citations(citing_paper_id);
CREATE INDEX IF NOT EXISTS idx_citations_cited ON citations(cited_paper_id);

-- =========================================================================
-- INGESTION JOBS TABLE (Async tracking)
-- =========================================================================
CREATE TABLE IF NOT EXISTS ingestion_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    paper_id UUID REFERENCES papers(id) ON DELETE SET NULL,
    
    status TEXT NOT NULL CHECK (status IN ('pending', 'chunking', 'embedding', 'indexing', 'completed', 'failed')),
    
    chunks_total INT DEFAULT 0,
    chunks_processed INT DEFAULT 0,
    error_message TEXT,
    
    -- Idempotency key (unique per tenant)
    idempotency_key TEXT,
    
    -- Retry tracking
    attempt_count INT DEFAULT 0,
    next_retry_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    
    CONSTRAINT jobs_tenant_idempotency_unique UNIQUE(tenant_id, idempotency_key)
);

-- Indexes for ingestion jobs
CREATE INDEX IF NOT EXISTS idx_jobs_tenant_status ON ingestion_jobs(tenant_id, status);
CREATE INDEX IF NOT EXISTS idx_jobs_status ON ingestion_jobs(status);
CREATE INDEX IF NOT EXISTS idx_jobs_paper ON ingestion_jobs(paper_id);
CREATE INDEX IF NOT EXISTS idx_jobs_pending ON ingestion_jobs(status, next_retry_at) 
    WHERE status IN ('pending', 'failed');

-- =========================================================================
-- SESSIONS TABLE (Context Engine)
-- =========================================================================
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    
    -- Session state as JSONB for flexibility
    state JSONB DEFAULT '{}' NOT NULL,
    
    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    last_active_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    expires_at TIMESTAMPTZ DEFAULT (NOW() + INTERVAL '30 minutes') NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_tenant ON sessions(tenant_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);

-- =========================================================================
-- QUERY LOG TABLE (Analytics)
-- =========================================================================
CREATE TABLE IF NOT EXISTS query_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    session_id UUID REFERENCES sessions(id) ON DELETE SET NULL,
    
    query_text TEXT NOT NULL,
    query_hash TEXT NOT NULL,  -- For deduplication/caching
    
    search_mode TEXT NOT NULL,  -- 'vector', 'hybrid', 'bm25', 'intelligent'
    result_count INT NOT NULL,
    latency_ms INT NOT NULL,
    
    -- Feedback tracking
    clicked_results JSONB DEFAULT '[]',
    
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_query_logs_tenant ON query_logs(tenant_id, created_at);
CREATE INDEX IF NOT EXISTS idx_query_logs_hash ON query_logs(query_hash);

-- =========================================================================
-- USEFUL VIEWS
-- =========================================================================

-- Papers with chunk counts
CREATE OR REPLACE VIEW paper_summaries AS
SELECT 
    p.id,
    p.tenant_id,
    p.title,
    p.source,
    p.published_at,
    p.created_at,
    COUNT(c.id) AS chunk_count,
    COALESCE(SUM(c.token_count), 0) AS total_tokens
FROM papers p
LEFT JOIN chunks c ON p.id = c.paper_id
GROUP BY p.id;

-- Chunks needing re-embedding (model version change)
CREATE OR REPLACE VIEW chunks_needing_reembed AS
SELECT c.*, p.title AS paper_title, p.tenant_id
FROM chunks c
JOIN papers p ON c.paper_id = p.id
WHERE c.embedding_model != (SELECT name FROM embedding_models WHERE is_default = true LIMIT 1)
   OR c.embedding IS NULL;

-- Citation stats per paper
CREATE OR REPLACE VIEW citation_stats AS
SELECT 
    p.id AS paper_id,
    p.title,
    (SELECT COUNT(*) FROM citations WHERE citing_paper_id = p.id) AS outgoing_citations,
    (SELECT COUNT(*) FROM citations WHERE cited_paper_id = p.id) AS incoming_citations
FROM papers p;

-- Job status summary by tenant
CREATE OR REPLACE VIEW job_status_summary AS
SELECT 
    tenant_id,
    status,
    COUNT(*) AS job_count,
    AVG(EXTRACT(EPOCH FROM (completed_at - started_at))) AS avg_duration_seconds
FROM ingestion_jobs
WHERE started_at IS NOT NULL
GROUP BY tenant_id, status;

-- =========================================================================
-- FUNCTIONS
-- =========================================================================

-- Cleanup old completed jobs
CREATE OR REPLACE FUNCTION cleanup_old_jobs(retention_hours INT DEFAULT 24) 
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM ingestion_jobs 
    WHERE status IN ('completed', 'failed') 
      AND completed_at < NOW() - (retention_hours || ' hours')::INTERVAL;
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Cleanup expired sessions
CREATE OR REPLACE FUNCTION cleanup_expired_sessions() 
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM sessions WHERE expires_at < NOW();
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Update paper updated_at on modification
CREATE OR REPLACE FUNCTION update_paper_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER papers_update_timestamp
    BEFORE UPDATE ON papers
    FOR EACH ROW
    EXECUTE FUNCTION update_paper_timestamp();

-- =========================================================================
-- ROW LEVEL SECURITY (Multi-tenant isolation)
-- =========================================================================

-- Enable RLS on multi-tenant tables
ALTER TABLE papers ENABLE ROW LEVEL SECURITY;
ALTER TABLE chunks ENABLE ROW LEVEL SECURITY;
ALTER TABLE ingestion_jobs ENABLE ROW LEVEL SECURITY;
ALTER TABLE sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE query_logs ENABLE ROW LEVEL SECURITY;

-- Create policies (application sets tenant_id in session)
-- Note: Requires SET app.current_tenant = 'tenant-uuid' per connection

CREATE POLICY papers_tenant_isolation ON papers
    USING (tenant_id = current_setting('app.current_tenant')::UUID);

CREATE POLICY chunks_tenant_isolation ON chunks
    USING (paper_id IN (
        SELECT id FROM papers WHERE tenant_id = current_setting('app.current_tenant')::UUID
    ));

CREATE POLICY jobs_tenant_isolation ON ingestion_jobs
    USING (tenant_id = current_setting('app.current_tenant')::UUID);

CREATE POLICY sessions_tenant_isolation ON sessions
    USING (tenant_id = current_setting('app.current_tenant')::UUID);

CREATE POLICY query_logs_tenant_isolation ON query_logs
    USING (tenant_id = current_setting('app.current_tenant')::UUID);

-- =========================================================================
-- INITIAL DATA
-- =========================================================================

-- Create default tenant for development
INSERT INTO tenants (id, name, api_key_hash, rate_limit_rps)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'development',
    'dev-key-hash',
    1000
) ON CONFLICT (name) DO NOTHING;

COMMENT ON TABLE tenants IS 'Multi-tenant organization accounts';
COMMENT ON TABLE papers IS 'Research papers with metadata';
COMMENT ON TABLE chunks IS 'Text chunks with embeddings for vector search';
COMMENT ON TABLE citations IS 'Citation graph between papers';
COMMENT ON TABLE ingestion_jobs IS 'Async ingestion job tracking';
COMMENT ON TABLE sessions IS 'User session state for context engine';
COMMENT ON TABLE query_logs IS 'Query analytics and feedback tracking';
