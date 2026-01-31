-- Create approval_requests table for Protocol M operator approval workflow.
-- Approval requests are created when high-value actions require operator authorization.

-- Create action type enum
CREATE TYPE approval_action_type AS ENUM (
    'delegate',    -- Delegating authority to another DID
    'spend'        -- Spending credits beyond threshold
);

-- Create approval request status enum
CREATE TYPE approval_request_status AS ENUM (
    'pending',     -- Awaiting operator approval
    'approved',    -- Approved by operator
    'rejected',    -- Rejected by operator
    'expired'      -- Approval window expired
);

-- Create approval_requests table
CREATE TABLE IF NOT EXISTS approval_requests (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- DID of the operator who must approve this request
    operator_did TEXT NOT NULL,

    -- Reference to the bounty (nullable - only for spend actions)
    bounty_id UUID REFERENCES bounties(id) ON DELETE SET NULL,

    -- Type of action requiring approval
    action_type approval_action_type NOT NULL,

    -- Amount involved (for spend actions)
    amount NUMERIC(20, 8),

    -- Current status of the request
    status approval_request_status NOT NULL DEFAULT 'pending',

    -- Additional metadata (e.g., delegate_to_did, reason, etc.)
    metadata JSONB DEFAULT '{}',

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMP WITH TIME ZONE,

    -- Expiry time for the approval request (defaults to 24 hours)
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (NOW() + INTERVAL '24 hours'),

    -- DID of the agent/user who created the request
    requester_did TEXT NOT NULL,

    -- Resolution details (null until resolved)
    resolution_reason TEXT
);

-- Create indexes for common queries
CREATE INDEX idx_approval_requests_operator_did ON approval_requests(operator_did);
CREATE INDEX idx_approval_requests_status ON approval_requests(status);
CREATE INDEX idx_approval_requests_requester_did ON approval_requests(requester_did);
CREATE INDEX idx_approval_requests_bounty_id ON approval_requests(bounty_id);
CREATE INDEX idx_approval_requests_created_at ON approval_requests(created_at DESC);

-- Index for finding pending requests nearing expiry
CREATE INDEX idx_approval_requests_pending_expiry ON approval_requests(expires_at)
    WHERE status = 'pending';

-- Composite index for operator's pending requests
CREATE INDEX idx_approval_requests_operator_pending
    ON approval_requests(operator_did, status)
    WHERE status = 'pending';

-- Comments
COMMENT ON TABLE approval_requests IS 'Stores approval requests for high-value actions requiring operator authorization';
COMMENT ON COLUMN approval_requests.operator_did IS 'DID of the operator who must approve this request';
COMMENT ON COLUMN approval_requests.action_type IS 'Type of action: delegate (authority) or spend (credits)';
COMMENT ON COLUMN approval_requests.amount IS 'Amount of credits involved (for spend actions)';
COMMENT ON COLUMN approval_requests.expires_at IS 'Request expires if not resolved by this time';
