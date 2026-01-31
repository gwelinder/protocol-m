-- Migration: Create artifact_derivations table for tracking artifact attribution
-- US-006A: Create artifact_derivations table

-- Create artifact_derivations table
CREATE TABLE IF NOT EXISTS artifact_derivations (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- The artifact that is derived from another
    artifact_id UUID NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,

    -- The parent artifact this artifact was derived from
    derived_from_id UUID NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,

    -- When this derivation was recorded
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Unique constraint: an artifact can only derive from the same parent once
CREATE UNIQUE INDEX IF NOT EXISTS idx_artifact_derivations_unique
    ON artifact_derivations(artifact_id, derived_from_id);

-- Index for finding all parents of an artifact
CREATE INDEX IF NOT EXISTS idx_artifact_derivations_artifact
    ON artifact_derivations(artifact_id);

-- Index for finding all children of an artifact
CREATE INDEX IF NOT EXISTS idx_artifact_derivations_derived_from
    ON artifact_derivations(derived_from_id);

-- Comments
COMMENT ON TABLE artifact_derivations IS 'Tracks derivation relationships between artifacts for attribution';
COMMENT ON COLUMN artifact_derivations.id IS 'Unique identifier for this derivation record';
COMMENT ON COLUMN artifact_derivations.artifact_id IS 'The artifact that was derived from a parent';
COMMENT ON COLUMN artifact_derivations.derived_from_id IS 'The parent artifact this was derived from';
COMMENT ON COLUMN artifact_derivations.created_at IS 'When this derivation relationship was recorded';
