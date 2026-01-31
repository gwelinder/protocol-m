'use client'

import { useState, useCallback } from 'react'
import { truncateDid, formatTimestamp, copyToClipboard } from '@/lib/utils'

export interface IdentityBadgeProps {
  /** The full DID string (e.g., did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw) */
  did: string
  /** The timestamp when the DID was bound to the account */
  createdAt: Date | string
}

/**
 * IdentityBadge component displays a DID in a truncated format with:
 * - Middle truncation (e.g., did:key:z6Mk...Wp)
 * - Tooltip showing full DID on hover
 * - Click-to-copy functionality
 * - Binding timestamp display
 */
export function IdentityBadge({ did, createdAt }: IdentityBadgeProps) {
  const [showTooltip, setShowTooltip] = useState(false)
  const [copied, setCopied] = useState(false)

  const handleCopy = useCallback(async () => {
    const success = await copyToClipboard(did)
    if (success) {
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }
  }, [did])

  const truncatedDid = truncateDid(did)
  const formattedDate = formatTimestamp(createdAt)

  return (
    <div
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '8px',
        padding: '6px 12px',
        backgroundColor: '#f3f4f6',
        borderRadius: '6px',
        fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
        fontSize: '13px',
        position: 'relative',
        cursor: 'pointer',
      }}
      onMouseEnter={() => setShowTooltip(true)}
      onMouseLeave={() => setShowTooltip(false)}
      onClick={handleCopy}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault()
          handleCopy()
        }
      }}
      aria-label={`Copy DID: ${did}`}
    >
      {/* DID Display */}
      <span style={{ color: '#374151' }}>{truncatedDid}</span>

      {/* Timestamp */}
      <span style={{ color: '#9ca3af', fontSize: '11px' }}>
        Bound {formattedDate}
      </span>

      {/* Tooltip */}
      {showTooltip && (
        <div
          style={{
            position: 'absolute',
            bottom: '100%',
            left: '50%',
            transform: 'translateX(-50%)',
            marginBottom: '8px',
            padding: '8px 12px',
            backgroundColor: '#1f2937',
            color: '#f9fafb',
            borderRadius: '6px',
            fontSize: '12px',
            whiteSpace: 'nowrap',
            zIndex: 1000,
            maxWidth: '400px',
            wordBreak: 'break-all',
          }}
          role="tooltip"
        >
          {copied ? 'Copied!' : did}
          {/* Tooltip Arrow */}
          <div
            style={{
              position: 'absolute',
              top: '100%',
              left: '50%',
              transform: 'translateX(-50%)',
              borderWidth: '6px',
              borderStyle: 'solid',
              borderColor: '#1f2937 transparent transparent transparent',
            }}
          />
        </div>
      )}
    </div>
  )
}

export default IdentityBadge
