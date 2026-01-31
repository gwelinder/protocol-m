-- Migration: Create did_challenges table for secure DID binding
-- US-008B: Create did_challenges table

-- Create did_challenges table
CREATE TABLE IF NOT EXISTS did_challenges (
    -- Primary key: UUID for globally unique identification
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- User ID requesting the challenge
    -- Using UUID to match common user ID patterns
    user_id UUID NOT NULL,

    -- The challenge text (random bytes encoded as hex)
    challenge VARCHAR(128) NOT NULL,

    -- When this challenge expires (typically 10 minutes from creation)
    expires_at TIMESTAMPTZ NOT NULL,

    -- When this challenge was used (null if not yet used)
    used_at TIMESTAMPTZ,

    -- When this challenge was created
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index on challenge for quick lookup during verification
CREATE INDEX IF NOT EXISTS idx_did_challenges_challenge ON did_challenges(challenge);

-- Index on expires_at for efficient cleanup of expired challenges
CREATE INDEX IF NOT EXISTS idx_did_challenges_expires_at ON did_challenges(expires_at);

-- Index on user_id for rate limiting queries
CREATE INDEX IF NOT EXISTS idx_did_challenges_user_id ON did_challenges(user_id);

-- Comment on table
COMMENT ON TABLE did_challenges IS 'Challenges for secure DID binding flow';
COMMENT ON COLUMN did_challenges.id IS 'Unique identifier for this challenge record';
COMMENT ON COLUMN did_challenges.user_id IS 'User ID requesting the challenge';
COMMENT ON COLUMN did_challenges.challenge IS 'Random challenge bytes encoded as hex';
COMMENT ON COLUMN did_challenges.expires_at IS 'When this challenge expires';
COMMENT ON COLUMN did_challenges.used_at IS 'When this challenge was used (null if not yet used)';
COMMENT ON COLUMN did_challenges.created_at IS 'When this challenge was created';
