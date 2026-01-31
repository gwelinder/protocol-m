-- Create disputes table for Protocol M bounty dispute resolution.
-- Disputes allow challenging fraudulent or incorrect bounty submissions.

-- Create dispute status enum
CREATE TYPE dispute_status AS ENUM (
    'pending',      -- Dispute awaiting resolution
    'resolved',     -- Dispute has been resolved (submission upheld or rejected)
    'expired'       -- Dispute window expired without resolution
);

-- Create disputes table
CREATE TABLE IF NOT EXISTS disputes (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Reference to the bounty being disputed
    bounty_id UUID NOT NULL REFERENCES bounties(id) ON DELETE CASCADE,

    -- Reference to the specific submission being disputed
    submission_id UUID NOT NULL REFERENCES bounty_submissions(id) ON DELETE CASCADE,

    -- DID of the agent/user who initiated the dispute
    initiator_did TEXT NOT NULL,

    -- Reason for the dispute
    reason TEXT NOT NULL,

    -- Current status of the dispute
    status dispute_status NOT NULL DEFAULT 'pending',

    -- Amount staked by the initiator (10% of bounty reward)
    stake_amount NUMERIC(20, 8) NOT NULL,

    -- Reference to the escrow hold for the stake
    stake_escrow_id UUID REFERENCES escrow_holds(id),

    -- Resolution outcome (null until resolved)
    resolution_outcome TEXT,  -- 'uphold_submission' or 'reject_submission'

    -- DID of the arbiter who resolved the dispute (null until resolved)
    resolver_did TEXT,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMP WITH TIME ZONE,

    -- Deadline for the dispute (7 days from creation)
    dispute_deadline TIMESTAMP WITH TIME ZONE NOT NULL
);

-- Create indexes for common queries
CREATE INDEX idx_disputes_bounty_id ON disputes(bounty_id);
CREATE INDEX idx_disputes_submission_id ON disputes(submission_id);
CREATE INDEX idx_disputes_initiator_did ON disputes(initiator_did);
CREATE INDEX idx_disputes_status ON disputes(status);

-- Index for finding pending disputes nearing deadline
CREATE INDEX idx_disputes_pending_deadline ON disputes(dispute_deadline)
    WHERE status = 'pending';

-- Unique constraint: only one active dispute per submission
CREATE UNIQUE INDEX idx_disputes_unique_pending_per_submission
    ON disputes(submission_id)
    WHERE status = 'pending';

-- Comments
COMMENT ON TABLE disputes IS 'Stores disputes against bounty submissions for fraud prevention';
COMMENT ON COLUMN disputes.stake_amount IS 'Amount staked by initiator (10% of bounty reward)';
COMMENT ON COLUMN disputes.dispute_deadline IS 'Deadline for dispute resolution (7 days from creation)';
