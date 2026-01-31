-- Create bounties table for task posting
-- Part of Protocol M bounty marketplace system

-- Create closure_type enum for bounty completion methods
CREATE TYPE bounty_closure_type AS ENUM ('tests', 'quorum', 'requester');

-- Create bounty_status enum for bounty lifecycle
CREATE TYPE bounty_status AS ENUM ('open', 'in_progress', 'completed', 'cancelled');

-- Create bounties table
CREATE TABLE IF NOT EXISTS bounties (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Poster information (DID of the agent/user posting the bounty)
    poster_did TEXT NOT NULL,

    -- Bounty details
    title TEXT NOT NULL,
    description TEXT NOT NULL,

    -- Reward amount in M-credits (NUMERIC(20,8) for precision)
    reward_credits NUMERIC(20, 8) NOT NULL CHECK (reward_credits > 0),

    -- How the bounty is closed/verified
    closure_type bounty_closure_type NOT NULL,

    -- Current status
    status bounty_status NOT NULL DEFAULT 'open',

    -- Additional metadata (JSONB for flexibility)
    -- For tests: { "eval_harness_hash": "sha256:..." }
    -- For quorum: { "reviewer_count": 3, "min_reviewer_rep": 100 }
    -- For requester: {} (manual approval)
    metadata JSONB NOT NULL DEFAULT '{}',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Deadline for bounty completion (nullable for no deadline)
    deadline TIMESTAMPTZ
);

-- Indexes for common queries
-- Index on poster_did for finding bounties by poster
CREATE INDEX idx_bounties_poster_did ON bounties(poster_did);

-- Index on status for filtering by status
CREATE INDEX idx_bounties_status ON bounties(status);

-- Index on deadline for finding expiring bounties
CREATE INDEX idx_bounties_deadline ON bounties(deadline) WHERE deadline IS NOT NULL;

-- Index on created_at for sorting by newest
CREATE INDEX idx_bounties_created_at ON bounties(created_at DESC);

-- Composite index for marketplace listing (open bounties sorted by creation)
CREATE INDEX idx_bounties_open_listing ON bounties(status, created_at DESC) WHERE status = 'open';

-- Trigger to auto-update updated_at timestamp
CREATE OR REPLACE FUNCTION update_bounties_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER bounties_updated_at_trigger
    BEFORE UPDATE ON bounties
    FOR EACH ROW
    EXECUTE FUNCTION update_bounties_updated_at();

-- Comments for documentation
COMMENT ON TABLE bounties IS 'Bounties table for Protocol M task marketplace';
COMMENT ON COLUMN bounties.poster_did IS 'DID of the agent/user who posted the bounty';
COMMENT ON COLUMN bounties.reward_credits IS 'Amount of M-credits offered as reward';
COMMENT ON COLUMN bounties.closure_type IS 'How bounty completion is verified: tests (automated), quorum (reviewers), or requester (manual)';
COMMENT ON COLUMN bounties.status IS 'Current bounty lifecycle status';
COMMENT ON COLUMN bounties.metadata IS 'Additional closure-type specific configuration';
COMMENT ON COLUMN bounties.deadline IS 'Optional deadline for bounty completion';
