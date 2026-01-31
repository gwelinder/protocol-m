-- Create submission status enum
CREATE TYPE submission_status AS ENUM ('pending', 'approved', 'rejected');

-- Create bounty_submissions table
CREATE TABLE IF NOT EXISTS bounty_submissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bounty_id UUID NOT NULL REFERENCES bounties(id) ON DELETE CASCADE,
    submitter_did TEXT NOT NULL,
    artifact_hash TEXT NOT NULL,
    signature_envelope JSONB NOT NULL,
    execution_receipt JSONB,
    status submission_status NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index on bounty_id for finding submissions for a bounty
CREATE INDEX idx_bounty_submissions_bounty_id ON bounty_submissions(bounty_id);

-- Index on submitter_did for finding submissions by a specific agent
CREATE INDEX idx_bounty_submissions_submitter_did ON bounty_submissions(submitter_did);

-- Index on status for filtering by approval state
CREATE INDEX idx_bounty_submissions_status ON bounty_submissions(status);

-- Composite index for pending submissions per bounty (common query)
CREATE INDEX idx_bounty_submissions_pending ON bounty_submissions(bounty_id, created_at DESC)
    WHERE status = 'pending';

COMMENT ON TABLE bounty_submissions IS 'Submissions of work for bounties';
COMMENT ON COLUMN bounty_submissions.artifact_hash IS 'SHA-256 hash of the submitted artifact';
COMMENT ON COLUMN bounty_submissions.signature_envelope IS 'Full SignatureEnvelopeV1 JSON for the submission';
COMMENT ON COLUMN bounty_submissions.execution_receipt IS 'Optional execution receipt for test-based bounties';
