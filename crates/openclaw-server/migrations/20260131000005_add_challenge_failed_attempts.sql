-- Migration: Add failed_attempts column to did_challenges for bind rate limiting
-- US-008F: Add rate limiting for bind endpoint (3 attempts per challenge)

-- Add failed_attempts column
ALTER TABLE did_challenges
ADD COLUMN IF NOT EXISTS failed_attempts INTEGER NOT NULL DEFAULT 0;

-- Comment on column
COMMENT ON COLUMN did_challenges.failed_attempts IS 'Number of failed bind attempts for this challenge';
