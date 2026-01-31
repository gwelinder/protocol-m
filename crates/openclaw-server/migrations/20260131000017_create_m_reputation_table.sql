-- Migration: Create m_reputation table for tracking agent/user reputation scores
-- Part of US-016A: Implement reputation calculation

-- Create the m_reputation table to store aggregated reputation scores
CREATE TABLE IF NOT EXISTS m_reputation (
    -- Use DID as primary key since reputation is per-identity
    did TEXT PRIMARY KEY,

    -- Total reputation score (accumulated over time, subject to decay)
    total_rep NUMERIC(20, 8) NOT NULL DEFAULT 0.00000000,

    -- Decay factor applied since last update (0.99^months since last decay)
    decay_factor NUMERIC(10, 8) NOT NULL DEFAULT 1.00000000,

    -- When the reputation was last updated
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- When this record was created
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT reputation_non_negative CHECK (total_rep >= 0),
    CONSTRAINT decay_factor_range CHECK (decay_factor > 0 AND decay_factor <= 1)
);

-- Index for efficient DID lookups (though PK already handles this)
-- Add index on last_updated for batch decay processing
CREATE INDEX IF NOT EXISTS idx_m_reputation_last_updated ON m_reputation(last_updated);

-- Comments for documentation
COMMENT ON TABLE m_reputation IS 'Stores aggregated reputation scores for Protocol M identities';
COMMENT ON COLUMN m_reputation.did IS 'DID (Decentralized Identifier) of the agent/user';
COMMENT ON COLUMN m_reputation.total_rep IS 'Current total reputation score after decay';
COMMENT ON COLUMN m_reputation.decay_factor IS 'Current decay multiplier (0.99^months since start)';
COMMENT ON COLUMN m_reputation.last_updated IS 'When reputation was last calculated/updated';
