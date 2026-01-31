-- Migration: Create m_credits_ledger table for event sourcing of credit transactions
-- US-012B: Create m_credits_ledger table for event sourcing

-- Create event type enum for the ledger
CREATE TYPE m_credits_event_type AS ENUM (
    'mint',      -- New credits created (from purchase or reward)
    'burn',      -- Credits destroyed (refund or expiry)
    'transfer',  -- Credits moved between DIDs
    'hold',      -- Credits reserved (pending transaction)
    'release'    -- Credits released from hold
);

-- Create m_credits_ledger table
CREATE TABLE IF NOT EXISTS m_credits_ledger (
    -- Primary key: UUID for globally unique identification
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- Type of event (mint, burn, transfer, hold, release)
    event_type m_credits_event_type NOT NULL,

    -- Source DID (null for mint events)
    from_did VARCHAR(256),

    -- Destination DID (null for burn events)
    to_did VARCHAR(256),

    -- Amount of credits in the transaction (20 digits, 8 decimal places)
    amount NUMERIC(20, 8) NOT NULL,

    -- Additional metadata (JSONB for flexible storage)
    -- Can include: reason, external_ref, invoice_id, etc.
    metadata JSONB DEFAULT '{}',

    -- Record creation timestamp (immutable)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT amount_positive CHECK (amount > 0),
    CONSTRAINT ledger_has_from_or_to CHECK (from_did IS NOT NULL OR to_did IS NOT NULL)
);

-- Index on event_type for filtering by transaction type
CREATE INDEX IF NOT EXISTS idx_m_credits_ledger_event_type ON m_credits_ledger(event_type);

-- Index on from_did for querying outgoing transactions
CREATE INDEX IF NOT EXISTS idx_m_credits_ledger_from_did ON m_credits_ledger(from_did);

-- Index on to_did for querying incoming transactions
CREATE INDEX IF NOT EXISTS idx_m_credits_ledger_to_did ON m_credits_ledger(to_did);

-- Index on created_at for time-based queries and ordering
CREATE INDEX IF NOT EXISTS idx_m_credits_ledger_created_at ON m_credits_ledger(created_at);

-- Composite index for DID history queries (all transactions for a DID)
CREATE INDEX IF NOT EXISTS idx_m_credits_ledger_did_history
    ON m_credits_ledger(created_at DESC)
    WHERE from_did IS NOT NULL OR to_did IS NOT NULL;

-- Comments
COMMENT ON TABLE m_credits_ledger IS 'Immutable ledger of all M-credit transactions for event sourcing';
COMMENT ON COLUMN m_credits_ledger.id IS 'Unique identifier for this ledger entry';
COMMENT ON COLUMN m_credits_ledger.event_type IS 'Type of credit event (mint, burn, transfer, hold, release)';
COMMENT ON COLUMN m_credits_ledger.from_did IS 'Source DID (null for mint events)';
COMMENT ON COLUMN m_credits_ledger.to_did IS 'Destination DID (null for burn events)';
COMMENT ON COLUMN m_credits_ledger.amount IS 'Amount of credits in this transaction';
COMMENT ON COLUMN m_credits_ledger.metadata IS 'Additional transaction metadata (JSONB)';
COMMENT ON COLUMN m_credits_ledger.created_at IS 'When this event occurred (immutable)';
COMMENT ON TYPE m_credits_event_type IS 'Types of M-credit ledger events';
