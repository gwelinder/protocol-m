-- Add pending_approval status to bounty_status enum
-- This status is used when a high-value bounty requires operator approval before posting

ALTER TYPE bounty_status ADD VALUE IF NOT EXISTS 'pending_approval';
