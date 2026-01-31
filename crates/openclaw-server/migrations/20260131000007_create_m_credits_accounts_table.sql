-- Migration: Create m_credits_accounts table for tracking M-credit balances
-- US-012A: Create m_credits_accounts table

-- Create m_credits_accounts table
CREATE TABLE IF NOT EXISTS m_credits_accounts (
    -- Primary key: UUID for globally unique identification
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- DID of the account holder (did:key:z6Mk...)
    -- Each DID can have only one credit account
    did VARCHAR(256) NOT NULL UNIQUE,

    -- Current balance of M-credits (20 digits, 8 decimal places)
    -- Supports values up to 999,999,999,999.99999999
    balance NUMERIC(20, 8) NOT NULL DEFAULT 0.00000000,

    -- Promotional/bonus balance (20 digits, 8 decimal places)
    -- Separate from main balance for tracking purposes
    promo_balance NUMERIC(20, 8) NOT NULL DEFAULT 0.00000000,

    -- Record creation timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Last update timestamp
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT balance_non_negative CHECK (balance >= 0),
    CONSTRAINT promo_balance_non_negative CHECK (promo_balance >= 0)
);

-- Index on did for fast lookup by account holder
CREATE INDEX IF NOT EXISTS idx_m_credits_accounts_did ON m_credits_accounts(did);

-- Trigger to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_m_credits_accounts_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_m_credits_accounts_updated_at
    BEFORE UPDATE ON m_credits_accounts
    FOR EACH ROW
    EXECUTE FUNCTION update_m_credits_accounts_updated_at();

-- Comments
COMMENT ON TABLE m_credits_accounts IS 'Stores M-credit account balances for each DID';
COMMENT ON COLUMN m_credits_accounts.id IS 'Unique identifier for this account record';
COMMENT ON COLUMN m_credits_accounts.did IS 'DID of the account holder (did:key:z6Mk...)';
COMMENT ON COLUMN m_credits_accounts.balance IS 'Current balance of M-credits';
COMMENT ON COLUMN m_credits_accounts.promo_balance IS 'Promotional/bonus balance of M-credits';
COMMENT ON COLUMN m_credits_accounts.created_at IS 'When this account was created';
COMMENT ON COLUMN m_credits_accounts.updated_at IS 'When this account was last updated';
