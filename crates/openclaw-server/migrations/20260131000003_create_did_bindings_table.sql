-- Migration: Create did_bindings table for linking DIDs to user accounts
-- US-008A: Create did_bindings table

-- Create did_bindings table
CREATE TABLE IF NOT EXISTS did_bindings (
    -- Primary key: UUID for globally unique identification
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- User ID (FK to users table when created)
    -- Using UUID to match common user ID patterns
    user_id UUID NOT NULL,

    -- DID of the bound identity (did:key:z6Mk...)
    did VARCHAR(256) NOT NULL,

    -- When this binding was created
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- When this binding was revoked (null if still active)
    revoked_at TIMESTAMPTZ
);

-- Index on user_id for querying bindings by user
CREATE INDEX IF NOT EXISTS idx_did_bindings_user_id ON did_bindings(user_id);

-- Index on did for looking up user by DID
CREATE INDEX IF NOT EXISTS idx_did_bindings_did ON did_bindings(did);

-- Unique constraint: a DID can only be bound to one user at a time (unless revoked)
-- This prevents the same DID from being bound to multiple accounts simultaneously
CREATE UNIQUE INDEX IF NOT EXISTS idx_did_bindings_did_active
    ON did_bindings(did)
    WHERE revoked_at IS NULL;

-- Comment on table
COMMENT ON TABLE did_bindings IS 'Links DIDs (decentralized identifiers) to user accounts';
COMMENT ON COLUMN did_bindings.id IS 'Unique identifier for this binding record';
COMMENT ON COLUMN did_bindings.user_id IS 'User ID that this DID is bound to';
COMMENT ON COLUMN did_bindings.did IS 'DID of the bound identity (did:key:z6Mk...)';
COMMENT ON COLUMN did_bindings.created_at IS 'When this binding was created';
COMMENT ON COLUMN did_bindings.revoked_at IS 'When this binding was revoked (null if still active)';
