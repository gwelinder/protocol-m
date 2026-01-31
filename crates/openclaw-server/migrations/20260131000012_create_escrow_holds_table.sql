-- Create escrow_holds table for bounty payment locking
-- Part of Protocol M bounty marketplace system

-- Create escrow_status enum for escrow lifecycle
CREATE TYPE escrow_status AS ENUM ('held', 'released', 'cancelled');

-- Create escrow_holds table
CREATE TABLE IF NOT EXISTS escrow_holds (
    -- Primary key
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Foreign key to bounty
    bounty_id UUID NOT NULL REFERENCES bounties(id) ON DELETE CASCADE,

    -- DID of the holder (the one who posted the bounty)
    holder_did TEXT NOT NULL,

    -- Amount held in escrow (NUMERIC(20,8) for M-credits precision)
    amount NUMERIC(20, 8) NOT NULL CHECK (amount > 0),

    -- Current status of the escrow
    status escrow_status NOT NULL DEFAULT 'held',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    released_at TIMESTAMPTZ
);

-- Indexes for common queries
-- Index on bounty_id for finding escrow by bounty
CREATE INDEX idx_escrow_holds_bounty_id ON escrow_holds(bounty_id);

-- Index on holder_did for finding escrow by holder
CREATE INDEX idx_escrow_holds_holder_did ON escrow_holds(holder_did);

-- Index on status for filtering by status
CREATE INDEX idx_escrow_holds_status ON escrow_holds(status);

-- Composite index for active escrows by holder
CREATE INDEX idx_escrow_holds_holder_active ON escrow_holds(holder_did, status) WHERE status = 'held';

-- Comments for documentation
COMMENT ON TABLE escrow_holds IS 'Escrow holds table for locking bounty payments';
COMMENT ON COLUMN escrow_holds.bounty_id IS 'Reference to the bounty this escrow is for';
COMMENT ON COLUMN escrow_holds.holder_did IS 'DID of the agent/user who funded the escrow';
COMMENT ON COLUMN escrow_holds.amount IS 'Amount of M-credits held in escrow';
COMMENT ON COLUMN escrow_holds.status IS 'Current escrow status: held, released, or cancelled';
COMMENT ON COLUMN escrow_holds.released_at IS 'When the escrow was released or cancelled (null if still held)';
