'use client'

import { useState } from 'react'
import { truncateDid } from '@/lib/utils'

/** Verification status enum matching the server-side VerificationStatus */
export type VerificationStatus = 'none' | 'invalid' | 'valid_unbound' | 'valid_bound'

export interface VerifiedBadgeProps {
  /** The verification status of the post */
  status: VerificationStatus
  /** The DID of the signer (required for valid_unbound and valid_bound statuses) */
  verifiedDid?: string | null
}

/**
 * VerifiedBadge component displays verification status for signed posts.
 * - valid_bound: Green checkmark with "Verified" text
 * - valid_unbound: "Signed" text without checkmark (signature valid but DID not bound)
 * - none/invalid: Renders nothing
 *
 * Includes hover tooltip showing the signer DID.
 */
export function VerifiedBadge({ status, verifiedDid }: VerifiedBadgeProps) {
  const [showTooltip, setShowTooltip] = useState(false)

  // Don't render anything for none or invalid status
  if (status === 'none' || status === 'invalid') {
    return null
  }

  const isValidBound = status === 'valid_bound'
  const truncatedDid = verifiedDid ? truncateDid(verifiedDid) : null

  return (
    <div
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '4px',
        padding: '2px 8px',
        backgroundColor: isValidBound ? '#dcfce7' : '#f3f4f6',
        borderRadius: '4px',
        fontSize: '12px',
        fontWeight: 500,
        position: 'relative',
        cursor: verifiedDid ? 'pointer' : 'default',
      }}
      onMouseEnter={() => setShowTooltip(true)}
      onMouseLeave={() => setShowTooltip(false)}
      role="status"
      aria-label={
        isValidBound
          ? `Verified signature from ${verifiedDid || 'unknown signer'}`
          : `Signed by ${verifiedDid || 'unknown signer'} (not bound to account)`
      }
    >
      {/* Checkmark icon for valid_bound */}
      {isValidBound && (
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="#16a34a"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <polyline points="20 6 9 17 4 12" />
        </svg>
      )}

      {/* Status text */}
      <span style={{ color: isValidBound ? '#16a34a' : '#6b7280' }}>
        {isValidBound ? 'Verified' : 'Signed'}
      </span>

      {/* Tooltip with signer DID */}
      {showTooltip && verifiedDid && (
        <div
          style={{
            position: 'absolute',
            bottom: '100%',
            left: '50%',
            transform: 'translateX(-50%)',
            marginBottom: '6px',
            padding: '6px 10px',
            backgroundColor: '#1f2937',
            color: '#f9fafb',
            borderRadius: '4px',
            fontSize: '11px',
            whiteSpace: 'nowrap',
            zIndex: 1000,
          }}
          role="tooltip"
        >
          <div style={{ marginBottom: '2px', color: '#9ca3af' }}>
            {isValidBound ? 'Verified signer' : 'Signer (not bound)'}
          </div>
          <div style={{ fontFamily: 'ui-monospace, monospace' }}>
            {truncatedDid}
          </div>
          {/* Tooltip Arrow */}
          <div
            style={{
              position: 'absolute',
              top: '100%',
              left: '50%',
              transform: 'translateX(-50%)',
              borderWidth: '5px',
              borderStyle: 'solid',
              borderColor: '#1f2937 transparent transparent transparent',
            }}
          />
        </div>
      )}
    </div>
  )
}

export default VerifiedBadge
