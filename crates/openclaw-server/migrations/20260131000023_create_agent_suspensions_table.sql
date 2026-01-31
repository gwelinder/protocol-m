-- Create agent_suspensions table for Protocol M kill switch functionality.
-- Suspensions allow operators to emergency-stop runaway agents.

-- Create agent suspensions table
CREATE TABLE IF NOT EXISTS agent_suspensions (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- DID of the operator who initiated the suspension (the identity being suspended)
    operator_did TEXT NOT NULL,

    -- Reason for the suspension
    reason TEXT NOT NULL,

    -- When the agent was suspended
    suspended_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- When the agent was resumed (null if still suspended)
    resumed_at TIMESTAMP WITH TIME ZONE,

    -- Additional metadata (e.g., number of bounties cancelled, escrow refunded)
    metadata JSONB DEFAULT '{}',

    -- DID of the user/admin who resumed the agent (null if still suspended)
    resumed_by_did TEXT
);

-- Create indexes for common queries
CREATE INDEX idx_agent_suspensions_operator_did ON agent_suspensions(operator_did);
CREATE INDEX idx_agent_suspensions_suspended_at ON agent_suspensions(suspended_at);

-- Index for finding active suspensions (resumed_at IS NULL)
CREATE INDEX idx_agent_suspensions_active ON agent_suspensions(operator_did)
    WHERE resumed_at IS NULL;

-- Comments
COMMENT ON TABLE agent_suspensions IS 'Stores agent suspension records for kill switch functionality';
COMMENT ON COLUMN agent_suspensions.operator_did IS 'DID of the agent/identity that was suspended';
COMMENT ON COLUMN agent_suspensions.reason IS 'Reason for the emergency stop';
COMMENT ON COLUMN agent_suspensions.resumed_at IS 'When the suspension was lifted (null if still active)';
COMMENT ON COLUMN agent_suspensions.metadata IS 'Additional context (bounties cancelled, escrow refunded, etc.)';
