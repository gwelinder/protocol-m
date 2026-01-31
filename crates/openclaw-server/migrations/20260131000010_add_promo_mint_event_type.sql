-- Migration: Add promo_mint event type to m_credits_event_type enum
-- US-012F: Implement promo credit grants

-- Add the promo_mint value to the existing enum type
-- PostgreSQL requires ALTER TYPE to add new enum values
ALTER TYPE m_credits_event_type ADD VALUE IF NOT EXISTS 'promo_mint';

-- Update comment to reflect the new value
COMMENT ON TYPE m_credits_event_type IS 'Types of M-credit ledger events: mint, burn, transfer, hold, release, promo_mint';
