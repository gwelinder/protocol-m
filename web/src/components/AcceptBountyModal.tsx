'use client'

import { useState } from 'react'
import { truncateDid } from '@/lib/utils'
import { ClosureType } from './BountyCard'

/** Submission instructions from the accept API response */
export interface SubmissionInstructions {
  endpoint: string
  closureType: ClosureType
  requirements: {
    type: string
    description: string
    evalHarnessHash?: string
    reviewerCount?: number
    minReviewerReputation?: number
    requiredFields: string[]
  }
  deadline?: string | null
}

/** Props for AcceptBountyModal */
export interface AcceptBountyModalProps {
  /** Whether the modal is open */
  isOpen: boolean
  /** Handler to close the modal */
  onClose: () => void
  /** Bounty ID */
  bountyId: string
  /** Bounty title */
  bountyTitle: string
  /** Reward amount in M-credits */
  rewardCredits: string | number
  /** Closure type for the bounty */
  closureType: ClosureType
  /** Whether the user has a bound DID */
  hasBoundDid: boolean
  /** Handler to navigate to DID binding */
  onNavigateToBindDid: () => void
  /** Handler to confirm acceptance */
  onConfirmAccept: (bountyId: string) => Promise<SubmissionInstructions | null>
  /** Current DID if bound */
  userDid?: string
}

/**
 * Modal for accepting a bounty.
 * Shows bind DID prompt if user doesn't have one, otherwise shows acceptance confirmation.
 */
export function AcceptBountyModal({
  isOpen,
  onClose,
  bountyId,
  bountyTitle,
  rewardCredits,
  closureType,
  hasBoundDid,
  onNavigateToBindDid,
  onConfirmAccept,
  userDid,
}: AcceptBountyModalProps) {
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [submissionInstructions, setSubmissionInstructions] = useState<SubmissionInstructions | null>(null)

  const handleConfirm = async () => {
    setIsLoading(true)
    setError(null)

    try {
      const instructions = await onConfirmAccept(bountyId)
      if (instructions) {
        setSubmissionInstructions(instructions)
      } else {
        setError('Failed to accept bounty. Please try again.')
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An unexpected error occurred')
    } finally {
      setIsLoading(false)
    }
  }

  const handleClose = () => {
    setSubmissionInstructions(null)
    setError(null)
    onClose()
  }

  if (!isOpen) return null

  const formatReward = (credits: string | number): string => {
    const num = typeof credits === 'string' ? parseFloat(credits) : credits
    return num.toFixed(2)
  }

  const closureTypeLabels: Record<ClosureType, string> = {
    tests: 'Automated Tests',
    quorum: 'Peer Review',
    requester: 'Requester Approval',
  }

  return (
    <div
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.5)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
        padding: '16px',
      }}
      onClick={handleClose}
      role="dialog"
      aria-modal="true"
      aria-labelledby="accept-bounty-title"
    >
      <div
        style={{
          backgroundColor: '#ffffff',
          borderRadius: '12px',
          padding: '24px',
          maxWidth: '500px',
          width: '100%',
          maxHeight: '90vh',
          overflowY: 'auto',
          boxShadow: '0 20px 25px -5px rgba(0, 0, 0, 0.1)',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Show bind DID prompt if user doesn't have one */}
        {!hasBoundDid && (
          <>
            <h2
              id="accept-bounty-title"
              style={{
                margin: '0 0 16px 0',
                fontSize: '18px',
                fontWeight: 600,
                color: '#111827',
              }}
            >
              DID Required
            </h2>
            <p
              style={{
                margin: '0 0 16px 0',
                fontSize: '14px',
                color: '#6b7280',
                lineHeight: 1.6,
              }}
            >
              You need to bind a DID (Decentralized Identifier) to your account before you can accept bounties.
              Your DID is your cryptographic identity that will be used to sign your submissions.
            </p>
            <div
              style={{
                display: 'flex',
                gap: '12px',
                justifyContent: 'flex-end',
              }}
            >
              <button
                onClick={handleClose}
                style={{
                  padding: '8px 16px',
                  fontSize: '14px',
                  border: '1px solid #e5e7eb',
                  borderRadius: '6px',
                  backgroundColor: '#ffffff',
                  color: '#374151',
                  cursor: 'pointer',
                }}
              >
                Cancel
              </button>
              <button
                onClick={onNavigateToBindDid}
                style={{
                  padding: '8px 16px',
                  fontSize: '14px',
                  border: 'none',
                  borderRadius: '6px',
                  backgroundColor: '#6366f1',
                  color: '#ffffff',
                  cursor: 'pointer',
                }}
              >
                Bind DID
              </button>
            </div>
          </>
        )}

        {/* Show confirmation when user has bound DID */}
        {hasBoundDid && !submissionInstructions && (
          <>
            <h2
              id="accept-bounty-title"
              style={{
                margin: '0 0 16px 0',
                fontSize: '18px',
                fontWeight: 600,
                color: '#111827',
              }}
            >
              Accept Bounty
            </h2>

            <div
              style={{
                backgroundColor: '#f9fafb',
                borderRadius: '8px',
                padding: '16px',
                marginBottom: '16px',
              }}
            >
              <h3
                style={{
                  margin: '0 0 8px 0',
                  fontSize: '15px',
                  fontWeight: 600,
                  color: '#111827',
                }}
              >
                {bountyTitle}
              </h3>
              <div
                style={{
                  display: 'flex',
                  gap: '16px',
                  fontSize: '13px',
                  color: '#6b7280',
                }}
              >
                <span>
                  <strong style={{ color: '#059669' }}>{formatReward(rewardCredits)} M</strong>
                </span>
                <span>{closureTypeLabels[closureType]}</span>
              </div>
            </div>

            {userDid && (
              <p
                style={{
                  margin: '0 0 16px 0',
                  fontSize: '13px',
                  color: '#6b7280',
                }}
              >
                You will accept this bounty using your DID:{' '}
                <code
                  style={{
                    backgroundColor: '#f3f4f6',
                    padding: '2px 6px',
                    borderRadius: '4px',
                    fontSize: '12px',
                  }}
                  title={userDid}
                >
                  {truncateDid(userDid)}
                </code>
              </p>
            )}

            <p
              style={{
                margin: '0 0 16px 0',
                fontSize: '14px',
                color: '#6b7280',
                lineHeight: 1.6,
              }}
            >
              By accepting this bounty, you commit to completing the task. The bounty will be marked as
              &quot;in progress&quot; and you will receive submission instructions.
            </p>

            {error && (
              <div
                style={{
                  padding: '12px',
                  backgroundColor: '#fef2f2',
                  borderRadius: '6px',
                  marginBottom: '16px',
                  color: '#dc2626',
                  fontSize: '13px',
                }}
              >
                {error}
              </div>
            )}

            <div
              style={{
                display: 'flex',
                gap: '12px',
                justifyContent: 'flex-end',
              }}
            >
              <button
                onClick={handleClose}
                disabled={isLoading}
                style={{
                  padding: '8px 16px',
                  fontSize: '14px',
                  border: '1px solid #e5e7eb',
                  borderRadius: '6px',
                  backgroundColor: '#ffffff',
                  color: '#374151',
                  cursor: isLoading ? 'not-allowed' : 'pointer',
                  opacity: isLoading ? 0.5 : 1,
                }}
              >
                Cancel
              </button>
              <button
                onClick={handleConfirm}
                disabled={isLoading}
                style={{
                  padding: '8px 16px',
                  fontSize: '14px',
                  border: 'none',
                  borderRadius: '6px',
                  backgroundColor: isLoading ? '#a5b4fc' : '#6366f1',
                  color: '#ffffff',
                  cursor: isLoading ? 'not-allowed' : 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '6px',
                }}
              >
                {isLoading ? 'Accepting...' : 'Confirm Accept'}
              </button>
            </div>
          </>
        )}

        {/* Show submission instructions after successful acceptance */}
        {hasBoundDid && submissionInstructions && (
          <>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
                marginBottom: '16px',
              }}
            >
              {/* Success checkmark */}
              <div
                style={{
                  width: '24px',
                  height: '24px',
                  borderRadius: '50%',
                  backgroundColor: '#dcfce7',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                }}
              >
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="#059669"
                  strokeWidth="3"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              </div>
              <h2
                id="accept-bounty-title"
                style={{
                  margin: 0,
                  fontSize: '18px',
                  fontWeight: 600,
                  color: '#111827',
                }}
              >
                Bounty Accepted
              </h2>
            </div>

            <p
              style={{
                margin: '0 0 16px 0',
                fontSize: '14px',
                color: '#6b7280',
              }}
            >
              You have successfully accepted the bounty. Follow the instructions below to submit your work.
            </p>

            <div
              style={{
                backgroundColor: '#f9fafb',
                borderRadius: '8px',
                padding: '16px',
                marginBottom: '16px',
              }}
            >
              <h3
                style={{
                  margin: '0 0 12px 0',
                  fontSize: '14px',
                  fontWeight: 600,
                  color: '#111827',
                }}
              >
                Submission Requirements
              </h3>

              <p
                style={{
                  margin: '0 0 12px 0',
                  fontSize: '13px',
                  color: '#6b7280',
                }}
              >
                {submissionInstructions.requirements.description}
              </p>

              <div style={{ fontSize: '13px' }}>
                <p style={{ margin: '0 0 8px 0', color: '#374151', fontWeight: 500 }}>
                  Required fields:
                </p>
                <ul
                  style={{
                    margin: 0,
                    paddingLeft: '20px',
                    color: '#6b7280',
                  }}
                >
                  {submissionInstructions.requirements.requiredFields.map((field) => (
                    <li key={field} style={{ marginBottom: '4px' }}>
                      <code
                        style={{
                          backgroundColor: '#e5e7eb',
                          padding: '2px 6px',
                          borderRadius: '4px',
                          fontSize: '12px',
                        }}
                      >
                        {field}
                      </code>
                    </li>
                  ))}
                </ul>
              </div>

              {submissionInstructions.requirements.evalHarnessHash && (
                <div style={{ marginTop: '12px' }}>
                  <p style={{ margin: '0 0 4px 0', fontSize: '13px', color: '#374151', fontWeight: 500 }}>
                    Test Harness Hash:
                  </p>
                  <code
                    style={{
                      display: 'block',
                      backgroundColor: '#e5e7eb',
                      padding: '8px',
                      borderRadius: '4px',
                      fontSize: '11px',
                      wordBreak: 'break-all',
                      color: '#374151',
                    }}
                  >
                    {submissionInstructions.requirements.evalHarnessHash}
                  </code>
                </div>
              )}

              {submissionInstructions.deadline && (
                <p
                  style={{
                    margin: '12px 0 0 0',
                    fontSize: '13px',
                    color: '#dc2626',
                  }}
                >
                  Deadline: {new Date(submissionInstructions.deadline).toLocaleString()}
                </p>
              )}
            </div>

            <div
              style={{
                backgroundColor: '#eff6ff',
                borderRadius: '8px',
                padding: '16px',
                marginBottom: '16px',
              }}
            >
              <h3
                style={{
                  margin: '0 0 8px 0',
                  fontSize: '14px',
                  fontWeight: 600,
                  color: '#1e40af',
                }}
              >
                Submission Endpoint
              </h3>
              <code
                style={{
                  display: 'block',
                  backgroundColor: '#dbeafe',
                  padding: '8px',
                  borderRadius: '4px',
                  fontSize: '12px',
                  color: '#1e40af',
                }}
              >
                POST {submissionInstructions.endpoint}
              </code>
            </div>

            <div
              style={{
                display: 'flex',
                justifyContent: 'flex-end',
              }}
            >
              <button
                onClick={handleClose}
                style={{
                  padding: '8px 16px',
                  fontSize: '14px',
                  border: 'none',
                  borderRadius: '6px',
                  backgroundColor: '#6366f1',
                  color: '#ffffff',
                  cursor: 'pointer',
                }}
              >
                Close
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  )
}

export default AcceptBountyModal
