//! Reputation routes and handlers for Protocol M.
//!
//! This module implements reputation calculation, minting, and time decay.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{closure_type_to_weight, BountyClosureType};

/// Monthly decay factor for reputation (0.99 = 1% decay per month).
pub const MONTHLY_DECAY_RATE: f64 = 0.99;

/// Base reputation rate: 10% of reward credits.
pub const BASE_REPUTATION_RATE: f64 = 0.1;

/// Mints reputation for a DID after bounty completion.
///
/// This function:
/// 1. Ensures the DID has a reputation record (creates one if needed)
/// 2. Applies time decay to existing reputation
/// 3. Calculates weighted reputation based on closure type
/// 4. Records the reputation event in the ledger
/// 5. Updates the total reputation
///
/// # Closure type weights
/// - tests: 1.5x (automated verification is most reliable)
/// - quorum: 1.2x (peer review provides good signal)
/// - requester: 1.0x (single approver is baseline)
///
/// # Formula
/// reputation = reward_credits * 0.1 * closure_type_weight * reviewer_credibility
pub async fn mint_reputation(
    pool: &PgPool,
    did: &str,
    amount: BigDecimal,
    reason: &str,
    closure_type: BountyClosureType,
    reviewer_credibility: Option<f64>,
    bounty_id: Option<Uuid>,
    submission_id: Option<Uuid>,
) -> Result<BigDecimal, AppError> {
    // Calculate weights
    let closure_weight = closure_type_to_weight(closure_type);
    let reviewer_weight = reviewer_credibility.unwrap_or(1.0);
    let total_weight = closure_weight * reviewer_weight;

    // Calculate weighted reputation amount
    let weight_decimal =
        BigDecimal::try_from(total_weight).map_err(|_| AppError::Internal("Invalid weight".to_string()))?;
    let weighted_amount = &amount * &weight_decimal;

    // Step 1: Ensure reputation record exists (upsert)
    ensure_reputation_record(pool, did).await?;

    // Step 2: Apply time decay if needed
    apply_time_decay(pool, did).await?;

    // Step 3: Insert reputation event
    let closure_type_weight_decimal = BigDecimal::try_from(closure_weight)
        .map_err(|_| AppError::Internal("Invalid closure weight".to_string()))?;
    let reviewer_weight_decimal = BigDecimal::try_from(reviewer_weight)
        .map_err(|_| AppError::Internal("Invalid reviewer weight".to_string()))?;

    let closure_type_str = match closure_type {
        BountyClosureType::Tests => "tests",
        BountyClosureType::Quorum => "quorum",
        BountyClosureType::Requester => "requester",
    };

    sqlx::query(
        r#"
        INSERT INTO reputation_events (
            did, event_type, base_amount, closure_type_weight, reviewer_weight,
            weighted_amount, reason, closure_type, bounty_id, submission_id, metadata
        )
        VALUES ($1, 'bounty_completion', $2, $3, $4, $5, $6, $7, $8, $9, '{}')
        "#,
    )
    .bind(did)
    .bind(&amount)
    .bind(&closure_type_weight_decimal)
    .bind(&reviewer_weight_decimal)
    .bind(&weighted_amount)
    .bind(reason)
    .bind(closure_type_str)
    .bind(bounty_id)
    .bind(submission_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to insert reputation event: {}", e)))?;

    // Step 4: Update total reputation
    let new_total: (BigDecimal,) = sqlx::query_as(
        r#"
        UPDATE m_reputation
        SET total_rep = total_rep + $2,
            last_updated = NOW()
        WHERE did = $1
        RETURNING total_rep
        "#,
    )
    .bind(did)
    .bind(&weighted_amount)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update reputation: {}", e)))?;

    Ok(new_total.0)
}

/// Ensures a reputation record exists for the given DID.
/// Creates a new record with zero reputation if none exists.
async fn ensure_reputation_record(pool: &PgPool, did: &str) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO m_reputation (did, total_rep, decay_factor, last_updated)
        VALUES ($1, 0, 1.0, NOW())
        ON CONFLICT (did) DO NOTHING
        "#,
    )
    .bind(did)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to ensure reputation record: {}", e)))?;

    Ok(())
}

/// Applies time decay to a DID's reputation if needed.
///
/// Decay is 0.99 per month since last update.
/// Records a decay event in the ledger for audit purposes.
pub async fn apply_time_decay(pool: &PgPool, did: &str) -> Result<(), AppError> {
    // Get current reputation state
    let record: Option<(BigDecimal, BigDecimal, DateTime<Utc>)> = sqlx::query_as(
        "SELECT total_rep, decay_factor, last_updated FROM m_reputation WHERE did = $1",
    )
    .bind(did)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to fetch reputation: {}", e)))?;

    let (total_rep, _decay_factor, last_updated) = match record {
        Some(r) => r,
        None => return Ok(()), // No record to decay
    };

    // Calculate months since last update
    let now = Utc::now();
    let duration = now.signed_duration_since(last_updated);
    let days = duration.num_days().max(0) as u32;
    let months = days / 30;

    if months == 0 {
        return Ok(()); // No decay needed
    }

    // Calculate decay: rep *= 0.99^months
    let decay_multiplier = MONTHLY_DECAY_RATE.powi(months as i32);
    let decay_decimal = BigDecimal::try_from(decay_multiplier)
        .map_err(|_| AppError::Internal("Invalid decay multiplier".to_string()))?;

    let new_total = &total_rep * &decay_decimal;
    let decay_amount = &total_rep - &new_total;

    // Record decay event if significant
    if decay_amount > BigDecimal::from(0) {
        let negative_decay_amount = BigDecimal::from(0) - &decay_amount;
        sqlx::query(
            r#"
            INSERT INTO reputation_events (
                did, event_type, base_amount, closure_type_weight, reviewer_weight,
                weighted_amount, reason, metadata
            )
            VALUES ($1, 'decay', $2, 1.0, 1.0, $2, $3, $4)
            "#,
        )
        .bind(did)
        .bind(&negative_decay_amount)
        .bind(format!("Time decay: {} month(s)", months))
        .bind(serde_json::json!({ "months_decayed": months }))
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to record decay event: {}", e)))?;
    }

    // Update reputation with decayed value
    sqlx::query(
        r#"
        UPDATE m_reputation
        SET total_rep = $2,
            decay_factor = decay_factor * $3,
            last_updated = NOW()
        WHERE did = $1
        "#,
    )
    .bind(did)
    .bind(&new_total)
    .bind(&decay_decimal)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update decayed reputation: {}", e)))?;

    Ok(())
}

/// Gets the current reputation for a DID.
pub async fn get_reputation(pool: &PgPool, did: &str) -> Result<Option<BigDecimal>, AppError> {
    let record: Option<(BigDecimal,)> =
        sqlx::query_as("SELECT total_rep FROM m_reputation WHERE did = $1")
            .bind(did)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get reputation: {}", e)))?;

    Ok(record.map(|(rep,)| rep))
}

/// Gets the effective reputation after applying pending time decay.
/// Does not modify the database - just calculates.
pub async fn get_effective_reputation(
    pool: &PgPool,
    did: &str,
) -> Result<Option<BigDecimal>, AppError> {
    let record: Option<(BigDecimal, DateTime<Utc>)> =
        sqlx::query_as("SELECT total_rep, last_updated FROM m_reputation WHERE did = $1")
            .bind(did)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get reputation: {}", e)))?;

    match record {
        Some((total_rep, last_updated)) => {
            let now = Utc::now();
            let duration = now.signed_duration_since(last_updated);
            let days = duration.num_days().max(0) as u32;
            let months = days / 30;

            if months == 0 {
                return Ok(Some(total_rep));
            }

            let decay_multiplier = MONTHLY_DECAY_RATE.powi(months as i32);
            let decay_decimal = BigDecimal::try_from(decay_multiplier)
                .map_err(|_| AppError::Internal("Invalid decay multiplier".to_string()))?;

            Ok(Some(&total_rep * &decay_decimal))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monthly_decay_rate() {
        assert_eq!(MONTHLY_DECAY_RATE, 0.99);
    }

    #[test]
    fn test_base_reputation_rate() {
        assert_eq!(BASE_REPUTATION_RATE, 0.1);
    }

    #[test]
    fn test_decay_calculation_one_month() {
        let initial = 100.0;
        let decayed = initial * MONTHLY_DECAY_RATE.powi(1);
        assert_eq!(decayed, 99.0);
    }

    #[test]
    fn test_decay_calculation_twelve_months() {
        let initial = 100.0;
        let decayed = initial * MONTHLY_DECAY_RATE.powi(12);
        // After 12 months: 100 * 0.99^12 ≈ 88.64
        assert!((decayed - 88.64).abs() < 0.01);
    }

    #[test]
    fn test_decay_calculation_thirty_six_months() {
        let initial = 100.0;
        let decayed = initial * MONTHLY_DECAY_RATE.powi(36);
        // After 36 months: 100 * 0.99^36 ≈ 69.64
        assert!((decayed - 69.64).abs() < 0.01);
    }

    #[test]
    fn test_weighted_reputation_tests() {
        let base = 100.0;
        let weight = closure_type_to_weight(BountyClosureType::Tests);
        assert_eq!(base * weight, 150.0);
    }

    #[test]
    fn test_weighted_reputation_quorum() {
        let base = 100.0;
        let weight = closure_type_to_weight(BountyClosureType::Quorum);
        assert_eq!(base * weight, 120.0);
    }

    #[test]
    fn test_weighted_reputation_requester() {
        let base = 100.0;
        let weight = closure_type_to_weight(BountyClosureType::Requester);
        assert_eq!(base * weight, 100.0);
    }

    #[test]
    fn test_weighted_reputation_with_reviewer_credibility() {
        let base = 100.0;
        let closure_weight = closure_type_to_weight(BountyClosureType::Quorum);
        let reviewer_credibility = 1.5;
        let weighted = base * closure_weight * reviewer_credibility;
        // 100 * 1.2 * 1.5 = 180
        assert_eq!(weighted, 180.0);
    }

    #[test]
    fn test_reputation_formula() {
        // reward_credits * 0.1 * closure_type_weight * reviewer_credibility
        let reward_credits = 1000.0;
        let closure_weight = closure_type_to_weight(BountyClosureType::Tests);
        let reviewer_credibility = 1.0;

        let reputation = reward_credits * BASE_REPUTATION_RATE * closure_weight * reviewer_credibility;
        // 1000 * 0.1 * 1.5 * 1.0 = 150
        assert_eq!(reputation, 150.0);
    }
}
