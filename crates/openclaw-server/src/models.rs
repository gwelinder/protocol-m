//! Database models for Protocol M.

pub mod approval_request;
pub mod artifact;
pub mod artifact_derivation;
pub mod bounty;
pub mod bounty_submission;
pub mod dispute;
pub mod compute_provider;
pub mod did_binding;
pub mod did_challenge;
pub mod escrow_hold;
pub mod m_credits_account;
pub mod m_credits_ledger;
pub mod m_reputation;
pub mod post;
pub mod purchase_invoice;
pub mod redemption_receipt;
pub mod reputation_event;

pub use approval_request::{
    ApprovalActionType, ApprovalRequest, ApprovalRequestStatus, NewApprovalRequest,
    APPROVAL_WINDOW_HOURS,
};
pub use artifact::{Artifact, NewArtifact};
pub use artifact_derivation::{ArtifactDerivation, NewArtifactDerivation};
pub use bounty::{Bounty, BountyClosureType, BountyStatus, NewBounty};
pub use bounty_submission::{BountySubmission, NewBountySubmission, SubmissionStatus};
pub use dispute::{
    calculate_dispute_stake, Dispute, DisputeStatus, NewDispute, ResolutionOutcome,
    DISPUTE_STAKE_PERCENTAGE, DISPUTE_WINDOW_DAYS,
};
pub use compute_provider::{ComputeProvider, NewComputeProvider, ProviderType};
pub use did_binding::{DidBinding, NewDidBinding};
pub use did_challenge::{DidChallenge, NewDidChallenge};
pub use escrow_hold::{EscrowHold, EscrowStatus, NewEscrowHold};
pub use m_credits_account::{MCreditsAccount, NewMCreditsAccount};
pub use m_credits_ledger::{MCreditsEventType, MCreditsLedger, NewMCreditsLedger};
pub use post::{NewPost, Post, VerificationStatus};
pub use purchase_invoice::{InvoiceStatus, NewPurchaseInvoice, PaymentProvider, PurchaseInvoice};
pub use redemption_receipt::{NewRedemptionReceipt, RedemptionReceipt};
pub use m_reputation::{MReputation, NewMReputation};
pub use reputation_event::{
    closure_type_to_weight, NewReputationEvent, ReputationEvent, ReputationEventType,
    WEIGHT_QUORUM, WEIGHT_REQUESTER, WEIGHT_TESTS,
};
