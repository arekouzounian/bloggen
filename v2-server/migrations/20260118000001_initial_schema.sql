-- BlogGen v2 Server Initial Schema
-- Content-addressable AST storage with posts and version history

-- Content-addressed AST nodes
CREATE TABLE ast_nodes (
    hash BYTEA PRIMARY KEY,              -- Blake3 hash (32 bytes)
    node_data JSONB NOT NULL,            -- Serialized AstNode
    ref_count INTEGER DEFAULT 0,         -- Reference counting for GC
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CHECK (length(hash) = 32)
);

CREATE INDEX idx_ast_nodes_ref_count ON ast_nodes(ref_count);

-- Blog posts
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    slug VARCHAR(255) UNIQUE NOT NULL,

    -- Content
    ast_root BYTEA NOT NULL REFERENCES ast_nodes(hash),

    -- Metadata
    title VARCHAR(500),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    published BOOLEAN DEFAULT false,

    -- Caching
    html_cache TEXT,
    html_cache_updated_at TIMESTAMPTZ,

    CHECK (length(ast_root) = 32)
);

CREATE INDEX idx_posts_published ON posts(published);
CREATE INDEX idx_posts_updated_at ON posts(updated_at DESC);
CREATE INDEX idx_posts_slug ON posts(slug);

-- Version history (free with content-addressing!)
CREATE TABLE post_versions (
    id SERIAL PRIMARY KEY,
    post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    ast_root BYTEA NOT NULL REFERENCES ast_nodes(hash),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(post_id, version_number),
    CHECK (length(ast_root) = 32)
);

CREATE INDEX idx_post_versions_post_id ON post_versions(post_id);

-- Function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_posts_updated_at BEFORE UPDATE ON posts
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
