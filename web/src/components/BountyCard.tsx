'use client'

import { truncateDid, formatTimestamp } from '@/lib/utils'

/** How a bounty's completion is verified */
export type ClosureType = 'tests' | 'quorum' | 'requester'

/** Possible states of a bounty */
export type BountyStatus = 'open' | 'in_progress' | 'completed' | 'cancelled'

export interface BountyCardProps {
  /** Unique identifier for the bounty */
  id: string
  /** Title of the bounty */
  title: string
  /** Detailed description of the task */
  description: string
  /** Amount of M-credits offered as reward */
  rewardCredits: string | number
  /** DID of the agent/user who posted the bounty */
  posterDid: string
  /** How bounty completion is verified */
  closureType: ClosureType
  /** Current status of the bounty */
  status: BountyStatus
  /** When this bounty was created */
  createdAt: string | Date
  /** Optional deadline for bounty completion */
  deadline?: string | Date | null
  /** Optional click handler for viewing details */
  onClick?: () => void
  /** Optional handler for accepting the bounty */
  onAccept?: (bountyId: string) => void
  /** Whether the accept button should be disabled */
  acceptDisabled?: boolean
}

/**
 * Badge component for displaying closure type
 */
function ClosureTypeBadge({ type }: { type: ClosureType }) {
  const config = {
    tests: {
      label: 'Tests',
      color: '#059669',
      backgroundColor: '#ecfdf5',
    },
    quorum: {
      label: 'Quorum',
      color: '#7c3aed',
      backgroundColor: '#f5f3ff',
    },
    requester: {
      label: 'Requester',
      color: '#0284c7',
      backgroundColor: '#f0f9ff',
    },
  }

  const { label, color, backgroundColor } = config[type]

  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        padding: '2px 8px',
        borderRadius: '9999px',
        fontSize: '11px',
        fontWeight: 500,
        color,
        backgroundColor,
      }}
    >
      {label}
    </span>
  )
}

/**
 * Format reward credits for display
 */
function formatCredits(credits: string | number): string {
  const num = typeof credits === 'string' ? parseFloat(credits) : credits
  if (num >= 1000) {
    return `${(num / 1000).toFixed(1)}k`
  }
  return num.toFixed(2)
}

/**
 * Format deadline for display
 */
function formatDeadline(deadline: string | Date): string {
  const d = typeof deadline === 'string' ? new Date(deadline) : deadline
  const now = new Date()
  const diff = d.getTime() - now.getTime()
  const days = Math.floor(diff / (1000 * 60 * 60 * 24))
  const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60))

  if (diff < 0) {
    return 'Expired'
  }
  if (days > 7) {
    return formatTimestamp(d)
  }
  if (days > 0) {
    return `${days}d ${hours}h left`
  }
  if (hours > 0) {
    return `${hours}h left`
  }
  return 'Ending soon'
}

/**
 * BountyCard component displays a bounty listing with title, description,
 * reward, poster DID, deadline, and closure type badge.
 */
export function BountyCard({
  id,
  title,
  description,
  rewardCredits,
  posterDid,
  closureType,
  status,
  createdAt,
  deadline,
  onClick,
  onAccept,
  acceptDisabled,
}: BountyCardProps) {
  const truncatedDescription =
    description.length > 200 ? `${description.slice(0, 200)}...` : description

  const isExpired = deadline ? new Date(deadline) < new Date() : false

  return (
    <article
      data-bounty-id={id}
      onClick={onClick}
      style={{
        backgroundColor: '#ffffff',
        border: '1px solid #e5e7eb',
        borderRadius: '8px',
        padding: '16px',
        cursor: onClick ? 'pointer' : 'default',
        transition: 'border-color 0.15s ease, box-shadow 0.15s ease',
        opacity: isExpired || status !== 'open' ? 0.7 : 1,
      }}
      onMouseEnter={(e) => {
        if (onClick) {
          e.currentTarget.style.borderColor = '#d1d5db'
          e.currentTarget.style.boxShadow = '0 1px 3px rgba(0, 0, 0, 0.1)'
        }
      }}
      onMouseLeave={(e) => {
        if (onClick) {
          e.currentTarget.style.borderColor = '#e5e7eb'
          e.currentTarget.style.boxShadow = 'none'
        }
      }}
    >
      {/* Header: Title and closure type badge */}
      <header
        style={{
          display: 'flex',
          alignItems: 'flex-start',
          justifyContent: 'space-between',
          gap: '12px',
          marginBottom: '8px',
        }}
      >
        <h3
          style={{
            margin: 0,
            fontSize: '16px',
            fontWeight: 600,
            color: '#111827',
            lineHeight: 1.4,
          }}
        >
          {title}
        </h3>
        <ClosureTypeBadge type={closureType} />
      </header>

      {/* Description */}
      <p
        style={{
          margin: '0 0 12px 0',
          fontSize: '14px',
          lineHeight: 1.5,
          color: '#6b7280',
        }}
      >
        {truncatedDescription}
      </p>

      {/* Footer: Reward, poster, deadline */}
      <footer
        style={{
          display: 'flex',
          alignItems: 'center',
          flexWrap: 'wrap',
          gap: '12px',
          fontSize: '13px',
        }}
      >
        {/* Reward */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '4px',
            color: '#059669',
            fontWeight: 600,
          }}
        >
          {/* Credits icon */}
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            <circle cx="12" cy="12" r="10" />
            <path d="M16 8h-6a2 2 0 1 0 0 4h4a2 2 0 1 1 0 4H8" />
            <path d="M12 18V6" />
          </svg>
          <span>{formatCredits(rewardCredits)} M</span>
        </div>

        {/* Poster DID */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '4px',
            color: '#6b7280',
          }}
          title={posterDid}
        >
          {/* User icon */}
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            <path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2" />
            <circle cx="12" cy="7" r="4" />
          </svg>
          <span>{truncateDid(posterDid)}</span>
        </div>

        {/* Deadline */}
        {deadline && (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
              color: isExpired ? '#dc2626' : '#6b7280',
              marginLeft: 'auto',
            }}
          >
            {/* Clock icon */}
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <circle cx="12" cy="12" r="10" />
              <polyline points="12 6 12 12 16 14" />
            </svg>
            <span>{formatDeadline(deadline)}</span>
          </div>
        )}

        {/* Created date if no deadline */}
        {!deadline && (
          <time
            dateTime={typeof createdAt === 'string' ? createdAt : createdAt.toISOString()}
            style={{
              color: '#9ca3af',
              marginLeft: 'auto',
            }}
          >
            Posted {formatTimestamp(createdAt)}
          </time>
        )}
      </footer>

      {/* Accept button row - only show for open bounties */}
      {status === 'open' && !isExpired && onAccept && (
        <div
          style={{
            marginTop: '12px',
            paddingTop: '12px',
            borderTop: '1px solid #f3f4f6',
            display: 'flex',
            justifyContent: 'flex-end',
          }}
        >
          <button
            onClick={(e) => {
              e.stopPropagation()
              onAccept(id)
            }}
            disabled={acceptDisabled}
            style={{
              padding: '8px 16px',
              fontSize: '13px',
              fontWeight: 500,
              border: 'none',
              borderRadius: '6px',
              backgroundColor: acceptDisabled ? '#e5e7eb' : '#6366f1',
              color: acceptDisabled ? '#9ca3af' : '#ffffff',
              cursor: acceptDisabled ? 'not-allowed' : 'pointer',
              transition: 'background-color 0.15s ease',
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
            }}
            onMouseEnter={(e) => {
              if (!acceptDisabled) {
                e.currentTarget.style.backgroundColor = '#4f46e5'
              }
            }}
            onMouseLeave={(e) => {
              if (!acceptDisabled) {
                e.currentTarget.style.backgroundColor = '#6366f1'
              }
            }}
            aria-label={`Accept bounty: ${title}`}
          >
            {/* Hand raised icon */}
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <path d="M18 11V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2" />
              <path d="M14 10V4a2 2 0 0 0-2-2a2 2 0 0 0-2 2v2" />
              <path d="M10 10.5V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2v8" />
              <path d="M18 8a2 2 0 1 1 4 0v6a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.86-5.99-2.34l-3.6-3.6a2 2 0 0 1 2.83-2.82L7 15" />
            </svg>
            Accept Bounty
          </button>
        </div>
      )}
    </article>
  )
}

export default BountyCard
