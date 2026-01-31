-- Migration: Create artifacts table for storing signed artifacts
-- US-005A: Create artifacts database table

-- Enable UUID extension if not already enabled
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create artifacts table
CREATE TABLE IF NOT EXISTS artifacts (
    -- Primary key: UUID for globally unique identification
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- SHA-256 hash of the artifact content (64 hex characters)
    hash VARCHAR(64) NOT NULL,

    -- DID of the signer (did:key:z6Mk...)
    did VARCHAR(256) NOT NULL,

    -- Timestamp from the signature envelope
    timestamp TIMESTAMPTZ NOT NULL,

    -- Additional metadata from the signature envelope (JSONB for flexibility)
    metadata JSONB NOT NULL DEFAULT '{}',

    -- Base64-encoded signature
    signature TEXT NOT NULL,

    -- Record creation timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index on hash for fast lookup by artifact content hash
CREATE INDEX IF NOT EXISTS idx_artifacts_hash ON artifacts(hash);

-- Index on did for querying artifacts by signer
CREATE INDEX IF NOT EXISTS idx_artifacts_did ON artifacts(did);

-- Index on timestamp for chronological queries
CREATE INDEX IF NOT EXISTS idx_artifacts_timestamp ON artifacts(timestamp);

-- Comment on table
COMMENT ON TABLE artifacts IS 'Stores registered signed artifacts with their signature envelopes';
COMMENT ON COLUMN artifacts.id IS 'Unique identifier for this artifact record';
COMMENT ON COLUMN artifacts.hash IS 'SHA-256 hash of the artifact content (hex-encoded)';
COMMENT ON COLUMN artifacts.did IS 'DID of the signer (did:key:z6Mk...)';
COMMENT ON COLUMN artifacts.timestamp IS 'Timestamp from the signature envelope';
COMMENT ON COLUMN artifacts.metadata IS 'Additional metadata from the signature envelope';
COMMENT ON COLUMN artifacts.signature IS 'Base64-encoded Ed25519 signature';
COMMENT ON COLUMN artifacts.created_at IS 'When this record was created';
