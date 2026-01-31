-- Create user_policies table for storing DID-linked spending policies
-- This enables the approval workflow for high-value bounty posting

CREATE TABLE IF NOT EXISTS user_policies (
    -- The DID that this policy belongs to (primary key)
    did TEXT PRIMARY KEY,

    -- Policy version (must be "1.0")
    version TEXT NOT NULL DEFAULT '1.0',

    -- Maximum credits that can be spent in a 24-hour rolling window
    max_spend_per_day NUMERIC(20, 8) NOT NULL DEFAULT 1000.00000000,

    -- Maximum credits that can be spent on a single bounty
    max_spend_per_bounty NUMERIC(20, 8) NOT NULL DEFAULT 500.00000000,

    -- Whether policy enforcement is active
    enabled BOOLEAN NOT NULL DEFAULT true,

    -- Approval tiers configuration (JSONB array)
    -- Each tier has: threshold, require_approval, approvers, timeout_hours, notification_channels
    approval_tiers JSONB NOT NULL DEFAULT '[{"threshold": 100, "require_approval": true, "approvers": [], "timeout_hours": 24, "notification_channels": []}]',

    -- Allowed delegates (DIDs that can act on behalf of this identity)
    allowed_delegates JSONB NOT NULL DEFAULT '[]',

    -- Emergency contact information (JSONB object with email, webhook)
    emergency_contact JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT user_policies_version_check CHECK (version = '1.0'),
    CONSTRAINT user_policies_max_spend_per_day_non_negative CHECK (max_spend_per_day >= 0),
    CONSTRAINT user_policies_max_spend_per_bounty_non_negative CHECK (max_spend_per_bounty >= 0)
);

-- Index for quick policy lookups by DID
CREATE INDEX IF NOT EXISTS idx_user_policies_did ON user_policies(did);

-- Index for finding enabled policies
CREATE INDEX IF NOT EXISTS idx_user_policies_enabled ON user_policies(did) WHERE enabled = true;

-- Trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_user_policies_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER user_policies_updated_at_trigger
    BEFORE UPDATE ON user_policies
    FOR EACH ROW
    EXECUTE FUNCTION update_user_policies_updated_at();
