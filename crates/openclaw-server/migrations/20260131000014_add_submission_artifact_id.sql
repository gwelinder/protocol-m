-- Add artifact_id column to bounty_submissions to link with registered artifacts
ALTER TABLE bounty_submissions
    ADD COLUMN artifact_id UUID REFERENCES artifacts(id) ON DELETE SET NULL;

-- Index on artifact_id for finding submissions by artifact
CREATE INDEX idx_bounty_submissions_artifact_id ON bounty_submissions(artifact_id) WHERE artifact_id IS NOT NULL;

COMMENT ON COLUMN bounty_submissions.artifact_id IS 'Reference to the registered artifact in ClawdHub (set on approval)';
