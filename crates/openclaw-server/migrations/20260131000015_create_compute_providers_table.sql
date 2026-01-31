-- Create compute_providers table for tracking credit redemption providers
-- US-015A: Create compute_providers table

-- Create provider type enum
CREATE TYPE provider_type AS ENUM (
    'openai',
    'anthropic',
    'gpu_provider'
);

-- Create compute_providers table
CREATE TABLE IF NOT EXISTS compute_providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Provider name (e.g., "OpenAI", "Anthropic Claude")
    name TEXT NOT NULL,
    -- Type of compute provider
    provider_type provider_type NOT NULL,
    -- API endpoint for this provider (nullable for internal providers)
    api_endpoint TEXT,
    -- Conversion rate: M-credits per unit of compute (e.g., 0.01 credits per token)
    -- NUMERIC(20,8) for high precision
    conversion_rate NUMERIC(20, 8) NOT NULL,
    -- Whether this provider is currently active
    is_active BOOLEAN NOT NULL DEFAULT true,
    -- When this provider was added
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- When this provider was last updated
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index on provider_type for filtering
CREATE INDEX idx_compute_providers_type ON compute_providers(provider_type);

-- Create index on is_active for active provider queries
CREATE INDEX idx_compute_providers_active ON compute_providers(is_active) WHERE is_active = true;

-- Create unique index on name to prevent duplicate providers
CREATE UNIQUE INDEX idx_compute_providers_name ON compute_providers(name);

-- Trigger to update updated_at automatically
CREATE OR REPLACE FUNCTION update_compute_providers_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_compute_providers_updated_at
    BEFORE UPDATE ON compute_providers
    FOR EACH ROW
    EXECUTE FUNCTION update_compute_providers_updated_at();

-- Insert default providers (OpenAI and Anthropic)
-- Conversion rates are placeholders - would be configured per deployment
-- Rate represents M-credits per 1000 tokens (typical API billing unit)
INSERT INTO compute_providers (name, provider_type, api_endpoint, conversion_rate)
VALUES
    ('OpenAI', 'openai', 'https://api.openai.com/v1', 1.00000000),
    ('Anthropic', 'anthropic', 'https://api.anthropic.com/v1', 1.00000000)
ON CONFLICT (name) DO NOTHING;

COMMENT ON TABLE compute_providers IS 'Compute providers for M-credit redemption';
COMMENT ON COLUMN compute_providers.conversion_rate IS 'M-credits required per unit of compute (e.g., per 1000 tokens)';
