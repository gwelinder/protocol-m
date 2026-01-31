'use client'

import { useState } from 'react'
import { IdentityBadge } from './IdentityBadge'

export interface BoundDid {
  did: string
  createdAt: Date | string
}

export interface ProfileIdentitiesProps {
  /** Array of bound DIDs for the user */
  dids: BoundDid[]
  /** Maximum number of DIDs to show before requiring expand (default 5) */
  maxVisible?: number
  /** URL for the "Bind DID" instructions page */
  bindInstructionsUrl?: string
}

/**
 * ProfileIdentities component displays all bound DIDs for a user profile.
 * Features:
 * - Shows max 5 DIDs by default, with expand option for more
 * - Displays "No identity bound" message when empty
 * - Includes "Bind DID" button linking to instructions
 */
export function ProfileIdentities({
  dids,
  maxVisible = 5,
  bindInstructionsUrl = '/bind-identity',
}: ProfileIdentitiesProps) {
  const [expanded, setExpanded] = useState(false)

  const hasMoreDids = dids.length > maxVisible
  const visibleDids = expanded ? dids : dids.slice(0, maxVisible)
  const hiddenCount = dids.length - maxVisible

  return (
    <div
      style={{
        padding: '16px',
        backgroundColor: '#ffffff',
        borderRadius: '8px',
        border: '1px solid #e5e7eb',
      }}
    >
      {/* Header */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: '12px',
        }}
      >
        <h3
          style={{
            margin: 0,
            fontSize: '14px',
            fontWeight: 600,
            color: '#374151',
          }}
        >
          Identities
        </h3>
        <a
          href={bindInstructionsUrl}
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
            padding: '6px 12px',
            backgroundColor: '#3b82f6',
            color: '#ffffff',
            borderRadius: '6px',
            fontSize: '13px',
            fontWeight: 500,
            textDecoration: 'none',
            cursor: 'pointer',
          }}
        >
          Bind DID
        </a>
      </div>

      {/* Empty State */}
      {dids.length === 0 && (
        <div
          style={{
            padding: '24px',
            textAlign: 'center',
            color: '#9ca3af',
            fontSize: '14px',
          }}
        >
          No identity bound
        </div>
      )}

      {/* DID List */}
      {dids.length > 0 && (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '8px',
          }}
        >
          {visibleDids.map((boundDid, index) => (
            <IdentityBadge
              key={`${boundDid.did}-${index}`}
              did={boundDid.did}
              createdAt={boundDid.createdAt}
            />
          ))}

          {/* Expand/Collapse Button */}
          {hasMoreDids && (
            <button
              onClick={() => setExpanded(!expanded)}
              style={{
                background: 'none',
                border: 'none',
                padding: '8px',
                color: '#3b82f6',
                fontSize: '13px',
                cursor: 'pointer',
                textAlign: 'center',
              }}
              type="button"
            >
              {expanded
                ? 'Show less'
                : `Show ${hiddenCount} more ${hiddenCount === 1 ? 'identity' : 'identities'}`}
            </button>
          )}
        </div>
      )}
    </div>
  )
}

export default ProfileIdentities
