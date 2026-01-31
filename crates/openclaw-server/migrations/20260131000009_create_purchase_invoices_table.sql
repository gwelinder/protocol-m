-- Migration: Create purchase_invoices table for tracking credit purchases
-- US-012C: Create purchase invoices table

-- Create payment provider enum
CREATE TYPE payment_provider AS ENUM (
    'stripe',    -- Stripe payments
    'usdc',      -- USDC stablecoin payments
    'apple_pay', -- Apple Pay (via Stripe)
    'manual'     -- Manual/admin credits
);

-- Create invoice status enum
CREATE TYPE invoice_status AS ENUM (
    'pending',   -- Payment initiated but not confirmed
    'completed', -- Payment confirmed, credits minted
    'failed'     -- Payment failed or cancelled
);

-- Create purchase_invoices table
CREATE TABLE IF NOT EXISTS purchase_invoices (
    -- Primary key: UUID for globally unique identification
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- User who initiated the purchase (no FK for now, users table doesn't exist yet)
    user_id UUID NOT NULL,

    -- Amount in USD (2 decimal places for cents precision)
    amount_usd NUMERIC(10, 2) NOT NULL,

    -- Amount in M-credits to be minted (8 decimal places)
    amount_credits NUMERIC(20, 8) NOT NULL,

    -- Payment provider used
    payment_provider payment_provider NOT NULL,

    -- External reference from payment provider (e.g., Stripe payment intent ID)
    external_ref VARCHAR(256),

    -- Current status of the invoice
    status invoice_status NOT NULL DEFAULT 'pending',

    -- Record creation timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Last update timestamp
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT amount_usd_positive CHECK (amount_usd > 0),
    CONSTRAINT amount_credits_positive CHECK (amount_credits > 0)
);

-- Index on user_id for querying user's purchase history
CREATE INDEX IF NOT EXISTS idx_purchase_invoices_user_id ON purchase_invoices(user_id);

-- Index on status for filtering by invoice status
CREATE INDEX IF NOT EXISTS idx_purchase_invoices_status ON purchase_invoices(status);

-- Index on created_at for time-based queries
CREATE INDEX IF NOT EXISTS idx_purchase_invoices_created_at ON purchase_invoices(created_at);

-- Index on external_ref for payment provider lookups
CREATE INDEX IF NOT EXISTS idx_purchase_invoices_external_ref ON purchase_invoices(external_ref);

-- Trigger to update updated_at on row modification
CREATE OR REPLACE FUNCTION update_purchase_invoices_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_purchase_invoices_updated_at
    BEFORE UPDATE ON purchase_invoices
    FOR EACH ROW
    EXECUTE FUNCTION update_purchase_invoices_updated_at();

-- Comments
COMMENT ON TABLE purchase_invoices IS 'Tracks credit purchase transactions from various payment providers';
COMMENT ON COLUMN purchase_invoices.id IS 'Unique identifier for this invoice';
COMMENT ON COLUMN purchase_invoices.user_id IS 'User who initiated the purchase';
COMMENT ON COLUMN purchase_invoices.amount_usd IS 'Amount charged in USD';
COMMENT ON COLUMN purchase_invoices.amount_credits IS 'Amount of M-credits to be minted';
COMMENT ON COLUMN purchase_invoices.payment_provider IS 'Payment provider used (stripe, usdc, etc.)';
COMMENT ON COLUMN purchase_invoices.external_ref IS 'External reference from payment provider';
COMMENT ON COLUMN purchase_invoices.status IS 'Current invoice status (pending, completed, failed)';
COMMENT ON COLUMN purchase_invoices.created_at IS 'When this invoice was created';
COMMENT ON COLUMN purchase_invoices.updated_at IS 'When this invoice was last updated';
COMMENT ON TYPE payment_provider IS 'Supported payment providers for credit purchases';
COMMENT ON TYPE invoice_status IS 'Possible states of a purchase invoice';
