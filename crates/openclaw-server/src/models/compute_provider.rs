//! Compute provider model for tracking credit redemption providers.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

/// Types of compute providers available for credit redemption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "provider_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    /// OpenAI API provider
    Openai,
    /// Anthropic API provider
    Anthropic,
    /// GPU compute provider
    GpuProvider,
}

/// Represents a compute provider for M-credit redemption.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ComputeProvider {
    /// Unique identifier for this provider.
    pub id: Uuid,
    /// Human-readable provider name.
    pub name: String,
    /// Type of compute provider.
    pub provider_type: ProviderType,
    /// API endpoint URL for this provider.
    pub api_endpoint: Option<String>,
    /// Conversion rate: M-credits per unit of compute.
    pub conversion_rate: BigDecimal,
    /// Whether this provider is currently active.
    pub is_active: bool,
    /// When this provider was created.
    pub created_at: DateTime<Utc>,
    /// When this provider was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Data required to create a new compute provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewComputeProvider {
    pub name: String,
    pub provider_type: ProviderType,
    pub api_endpoint: Option<String>,
    pub conversion_rate: BigDecimal,
}

impl ComputeProvider {
    /// Check if this provider is active and available for redemption.
    pub fn is_available(&self) -> bool {
        self.is_active
    }

    /// Calculate the credits required for a given number of compute units.
    pub fn credits_for_units(&self, units: &BigDecimal) -> BigDecimal {
        units * &self.conversion_rate
    }

    /// Check if the provider has a configured API endpoint.
    pub fn has_endpoint(&self) -> bool {
        self.api_endpoint.is_some()
    }
}

impl NewComputeProvider {
    /// Create a new OpenAI provider configuration.
    pub fn openai(name: &str, api_endpoint: &str, conversion_rate: BigDecimal) -> Self {
        Self {
            name: name.to_string(),
            provider_type: ProviderType::Openai,
            api_endpoint: Some(api_endpoint.to_string()),
            conversion_rate,
        }
    }

    /// Create a new Anthropic provider configuration.
    pub fn anthropic(name: &str, api_endpoint: &str, conversion_rate: BigDecimal) -> Self {
        Self {
            name: name.to_string(),
            provider_type: ProviderType::Anthropic,
            api_endpoint: Some(api_endpoint.to_string()),
            conversion_rate,
        }
    }

    /// Create a new GPU provider configuration.
    pub fn gpu(name: &str, api_endpoint: Option<&str>, conversion_rate: BigDecimal) -> Self {
        Self {
            name: name.to_string(),
            provider_type: ProviderType::GpuProvider,
            api_endpoint: api_endpoint.map(|s| s.to_string()),
            conversion_rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_provider_type_serialization() {
        assert_eq!(
            serde_json::to_string(&ProviderType::Openai).unwrap(),
            "\"openai\""
        );
        assert_eq!(
            serde_json::to_string(&ProviderType::Anthropic).unwrap(),
            "\"anthropic\""
        );
        assert_eq!(
            serde_json::to_string(&ProviderType::GpuProvider).unwrap(),
            "\"gpu_provider\""
        );
    }

    #[test]
    fn test_provider_type_deserialization() {
        assert_eq!(
            serde_json::from_str::<ProviderType>("\"openai\"").unwrap(),
            ProviderType::Openai
        );
        assert_eq!(
            serde_json::from_str::<ProviderType>("\"anthropic\"").unwrap(),
            ProviderType::Anthropic
        );
        assert_eq!(
            serde_json::from_str::<ProviderType>("\"gpu_provider\"").unwrap(),
            ProviderType::GpuProvider
        );
    }

    #[test]
    fn test_new_compute_provider_openai() {
        let rate = BigDecimal::from_str("1.00000000").unwrap();
        let provider = NewComputeProvider::openai("OpenAI", "https://api.openai.com/v1", rate.clone());

        assert_eq!(provider.name, "OpenAI");
        assert_eq!(provider.provider_type, ProviderType::Openai);
        assert_eq!(provider.api_endpoint, Some("https://api.openai.com/v1".to_string()));
        assert_eq!(provider.conversion_rate, rate);
    }

    #[test]
    fn test_new_compute_provider_anthropic() {
        let rate = BigDecimal::from_str("1.50000000").unwrap();
        let provider = NewComputeProvider::anthropic("Anthropic", "https://api.anthropic.com/v1", rate.clone());

        assert_eq!(provider.name, "Anthropic");
        assert_eq!(provider.provider_type, ProviderType::Anthropic);
        assert_eq!(provider.api_endpoint, Some("https://api.anthropic.com/v1".to_string()));
        assert_eq!(provider.conversion_rate, rate);
    }

    #[test]
    fn test_new_compute_provider_gpu() {
        let rate = BigDecimal::from_str("0.50000000").unwrap();
        let provider = NewComputeProvider::gpu("Local GPU", None, rate.clone());

        assert_eq!(provider.name, "Local GPU");
        assert_eq!(provider.provider_type, ProviderType::GpuProvider);
        assert_eq!(provider.api_endpoint, None);
        assert_eq!(provider.conversion_rate, rate);
    }

    #[test]
    fn test_compute_provider_is_available() {
        let now = Utc::now();
        let provider = ComputeProvider {
            id: Uuid::new_v4(),
            name: "Test Provider".to_string(),
            provider_type: ProviderType::Openai,
            api_endpoint: Some("https://api.example.com".to_string()),
            conversion_rate: BigDecimal::from_str("1.00000000").unwrap(),
            is_active: true,
            created_at: now,
            updated_at: now,
        };

        assert!(provider.is_available());
        assert!(provider.has_endpoint());
    }

    #[test]
    fn test_compute_provider_inactive() {
        let now = Utc::now();
        let provider = ComputeProvider {
            id: Uuid::new_v4(),
            name: "Inactive Provider".to_string(),
            provider_type: ProviderType::Anthropic,
            api_endpoint: None,
            conversion_rate: BigDecimal::from_str("1.00000000").unwrap(),
            is_active: false,
            created_at: now,
            updated_at: now,
        };

        assert!(!provider.is_available());
        assert!(!provider.has_endpoint());
    }

    #[test]
    fn test_credits_for_units() {
        let now = Utc::now();
        let provider = ComputeProvider {
            id: Uuid::new_v4(),
            name: "Test Provider".to_string(),
            provider_type: ProviderType::Openai,
            api_endpoint: Some("https://api.example.com".to_string()),
            conversion_rate: BigDecimal::from_str("0.01000000").unwrap(), // 0.01 credits per unit
            is_active: true,
            created_at: now,
            updated_at: now,
        };

        let units = BigDecimal::from_str("1000").unwrap(); // 1000 units
        let credits = provider.credits_for_units(&units);

        // 1000 units * 0.01 credits/unit = 10 credits
        assert_eq!(credits, BigDecimal::from_str("10.00000000").unwrap());
    }
}
