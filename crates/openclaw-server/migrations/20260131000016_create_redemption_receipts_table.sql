-- Migration: Create redemption_receipts table for tracking credit redemptions
-- US-015B: Implement credit redemption endpoint (receipts storage)

-- Create redemption_receipts table
CREATE TABLE IF NOT EXISTS redemption_receipts (
    -- Primary key: UUID for globally unique identification
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- DID of the user who redeemed credits
    user_did VARCHAR(256) NOT NULL,

    -- Reference to the compute provider used for redemption
    provider_id UUID NOT NULL REFERENCES compute_providers(id),

    -- Amount of M-credits redeemed
    amount_credits NUMERIC(20, 8) NOT NULL,

    -- Allocation ID from the provider (may be null if provider doesn't return one)
    allocation_id TEXT,

    -- Additional metadata (JSONB for flexible storage)
    -- Can include: provider response, usage quota, expiry, etc.
    metadata JSONB DEFAULT '{}',

    -- Record creation timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT amount_credits_positive CHECK (amount_credits > 0)
);

-- Index on user_did for querying redemptions by user
CREATE INDEX IF NOT EXISTS idx_redemption_receipts_user_did ON redemption_receipts(user_did);

-- Index on provider_id for querying redemptions by provider
CREATE INDEX IF NOT EXISTS idx_redemption_receipts_provider_id ON redemption_receipts(provider_id);

-- Index on created_at for time-based queries
CREATE INDEX IF NOT EXISTS idx_redemption_receipts_created_at ON redemption_receipts(created_at DESC);

-- Comments
COMMENT ON TABLE redemption_receipts IS 'Records of M-credit redemptions with compute providers';
COMMENT ON COLUMN redemption_receipts.id IS 'Unique identifier for this redemption receipt';
COMMENT ON COLUMN redemption_receipts.user_did IS 'DID of the user who redeemed credits';
COMMENT ON COLUMN redemption_receipts.provider_id IS 'Reference to the compute provider';
COMMENT ON COLUMN redemption_receipts.amount_credits IS 'Amount of M-credits redeemed';
COMMENT ON COLUMN redemption_receipts.allocation_id IS 'Allocation ID from the provider (if available)';
COMMENT ON COLUMN redemption_receipts.metadata IS 'Additional redemption metadata (JSONB)';
COMMENT ON COLUMN redemption_receipts.created_at IS 'When this redemption occurred';
