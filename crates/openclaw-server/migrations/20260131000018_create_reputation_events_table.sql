-- Migration: Create reputation_events table for event sourcing reputation changes
-- Part of US-016A: Implement reputation calculation

-- Enum for reputation event types
CREATE TYPE reputation_event_type AS ENUM (
    'bounty_completion',   -- Earned from completing a bounty
    'review_contribution', -- Earned from reviewing/validating work (quorum)
    'manual_adjustment',   -- Admin adjustment (corrections, disputes)
    'decay'                -- Time-based decay event
);

-- Create the reputation_events table (append-only ledger)
CREATE TABLE IF NOT EXISTS reputation_events (
    -- Unique identifier for this event
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- DID that received/lost the reputation
    did TEXT NOT NULL,

    -- Type of reputation event
    event_type reputation_event_type NOT NULL,

    -- Base amount before weighting (can be negative for decay/adjustment)
    base_amount NUMERIC(20, 8) NOT NULL,

    -- Closure type weight applied (1.0, 1.2, or 1.5)
    closure_type_weight NUMERIC(4, 2) NOT NULL DEFAULT 1.00,

    -- Reviewer credibility weight if applicable (for quorum)
    reviewer_weight NUMERIC(4, 2) NOT NULL DEFAULT 1.00,

    -- Final weighted amount (base_amount * closure_type_weight * reviewer_weight)
    weighted_amount NUMERIC(20, 8) NOT NULL,

    -- Reason description for this reputation change
    reason TEXT NOT NULL,

    -- Closure type that triggered this event (nullable for non-bounty events)
    closure_type TEXT,

    -- Related bounty ID if applicable
    bounty_id UUID,

    -- Related submission ID if applicable
    submission_id UUID,

    -- Additional metadata (JSONB for flexibility)
    metadata JSONB NOT NULL DEFAULT '{}',

    -- When this event occurred (immutable)
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Foreign key to m_reputation
    CONSTRAINT fk_reputation_events_did FOREIGN KEY (did) REFERENCES m_reputation(did) ON DELETE CASCADE
);

-- Index for DID history queries
CREATE INDEX IF NOT EXISTS idx_reputation_events_did ON reputation_events(did);

-- Index for time-based queries
CREATE INDEX IF NOT EXISTS idx_reputation_events_created_at ON reputation_events(created_at);

-- Composite index for DID + time queries
CREATE INDEX IF NOT EXISTS idx_reputation_events_did_created_at ON reputation_events(did, created_at DESC);

-- Index for bounty-related queries
CREATE INDEX IF NOT EXISTS idx_reputation_events_bounty_id ON reputation_events(bounty_id) WHERE bounty_id IS NOT NULL;

-- Comments for documentation
COMMENT ON TABLE reputation_events IS 'Append-only ledger of all reputation-affecting events';
COMMENT ON COLUMN reputation_events.base_amount IS 'Raw reputation amount before any weighting';
COMMENT ON COLUMN reputation_events.closure_type_weight IS 'Weight based on bounty closure type: tests=1.5, quorum=1.2, requester=1.0';
COMMENT ON COLUMN reputation_events.reviewer_weight IS 'Additional weight based on reviewer credibility (for quorum closures)';
COMMENT ON COLUMN reputation_events.weighted_amount IS 'Final reputation change: base_amount * closure_type_weight * reviewer_weight';
