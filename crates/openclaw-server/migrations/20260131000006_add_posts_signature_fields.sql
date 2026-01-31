-- Add signature-related fields to the posts table for Protocol M verification
-- US-010A: Signature fields for post verification

-- Create verification status enum
CREATE TYPE verification_status AS ENUM ('none', 'invalid', 'valid_unbound', 'valid_bound');

-- Add signature fields to posts table
-- Note: If posts table doesn't exist yet, this migration creates it
-- In production, this would ALTER an existing table
CREATE TABLE IF NOT EXISTS posts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add signature envelope JSON column (nullable - not all posts are signed)
ALTER TABLE posts ADD COLUMN IF NOT EXISTS signature_envelope_json JSONB;

-- Add verified DID column (nullable - populated after verification)
ALTER TABLE posts ADD COLUMN IF NOT EXISTS verified_did TEXT;

-- Add verification status column with default 'none'
ALTER TABLE posts ADD COLUMN IF NOT EXISTS verification_status verification_status NOT NULL DEFAULT 'none';

-- Index on verification_status for querying verified posts
CREATE INDEX IF NOT EXISTS idx_posts_verification_status ON posts(verification_status);

-- Index on verified_did for filtering by signer
CREATE INDEX IF NOT EXISTS idx_posts_verified_did ON posts(verified_did) WHERE verified_did IS NOT NULL;

-- Comment on columns for documentation
COMMENT ON COLUMN posts.signature_envelope_json IS 'Protocol M signature envelope as JSON (nullable)';
COMMENT ON COLUMN posts.verified_did IS 'DID of the verified signer (nullable, populated after verification)';
COMMENT ON COLUMN posts.verification_status IS 'Status of signature verification: none, invalid, valid_unbound, valid_bound';
