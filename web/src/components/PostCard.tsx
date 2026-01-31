'use client'

import { VerifiedBadge, VerificationStatus } from './VerifiedBadge'
import { SignatureEnvelope } from './SignatureModal'
import { formatTimestamp } from '@/lib/utils'

export interface PostCardProps {
  /** Unique identifier for the post */
  id: string
  /** The main content of the post */
  content: string
  /** Author display name */
  authorName: string
  /** Author username (optional) */
  authorUsername?: string
  /** Author avatar URL (optional) */
  authorAvatarUrl?: string
  /** When the post was created */
  createdAt: string | Date
  /** Verification status of the post signature */
  verificationStatus: VerificationStatus
  /** The DID that signed this post (if verified) */
  verifiedDid?: string | null
  /** The signature envelope JSON (for viewing in modal) */
  signatureEnvelope?: SignatureEnvelope | null
  /** Number of upvotes */
  upvotes?: number
  /** Number of comments */
  commentCount?: number
  /** Whether the current user has upvoted this post */
  hasUpvoted?: boolean
  /** Optional click handler for the card */
  onClick?: () => void
}

/**
 * PostCard component displays a post with author information, content,
 * and verification status.
 *
 * The VerifiedBadge is positioned next to the author name to indicate
 * whether the post has a valid cryptographic signature.
 */
export function PostCard({
  id,
  content,
  authorName,
  authorUsername,
  authorAvatarUrl,
  createdAt,
  verificationStatus,
  verifiedDid,
  signatureEnvelope,
  upvotes = 0,
  commentCount = 0,
  hasUpvoted = false,
  onClick,
}: PostCardProps) {
  const formattedDate = formatTimestamp(createdAt)

  return (
    <article
      data-post-id={id}
      onClick={onClick}
      style={{
        backgroundColor: '#ffffff',
        border: '1px solid #e5e7eb',
        borderRadius: '8px',
        padding: '16px',
        cursor: onClick ? 'pointer' : 'default',
        transition: 'border-color 0.15s ease, box-shadow 0.15s ease',
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
      {/* Header: Author info + verification badge */}
      <header
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '12px',
          marginBottom: '12px',
        }}
      >
        {/* Author avatar */}
        <div
          style={{
            width: '40px',
            height: '40px',
            borderRadius: '50%',
            backgroundColor: '#f3f4f6',
            backgroundImage: authorAvatarUrl ? `url(${authorAvatarUrl})` : undefined,
            backgroundSize: 'cover',
            backgroundPosition: 'center',
            flexShrink: 0,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#6b7280',
            fontSize: '16px',
            fontWeight: 600,
          }}
          aria-hidden="true"
        >
          {!authorAvatarUrl && authorName.charAt(0).toUpperCase()}
        </div>

        {/* Author name, username, and verification badge */}
        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              flexWrap: 'wrap',
            }}
          >
            <span
              style={{
                fontWeight: 600,
                color: '#111827',
                fontSize: '14px',
              }}
            >
              {authorName}
            </span>

            {/* Verified badge positioned next to author name */}
            <VerifiedBadge
              status={verificationStatus}
              verifiedDid={verifiedDid}
              signatureEnvelope={signatureEnvelope}
            />
          </div>

          {/* Username and timestamp */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              marginTop: '2px',
              fontSize: '12px',
              color: '#6b7280',
            }}
          >
            {authorUsername && <span>@{authorUsername}</span>}
            {authorUsername && <span aria-hidden="true">·</span>}
            <time dateTime={typeof createdAt === 'string' ? createdAt : createdAt.toISOString()}>
              {formattedDate}
            </time>
          </div>
        </div>
      </header>

      {/* Post content */}
      <div
        style={{
          fontSize: '14px',
          lineHeight: 1.6,
          color: '#374151',
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-word',
        }}
      >
        {content}
      </div>

      {/* Footer: Engagement stats */}
      <footer
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '16px',
          marginTop: '12px',
          paddingTop: '12px',
          borderTop: '1px solid #f3f4f6',
        }}
      >
        {/* Upvote button */}
        <button
          type="button"
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '4px',
            padding: '4px 8px',
            border: 'none',
            background: hasUpvoted ? '#fef2f2' : 'transparent',
            borderRadius: '4px',
            fontSize: '12px',
            color: hasUpvoted ? '#ef4444' : '#6b7280',
            cursor: 'pointer',
            transition: 'background-color 0.15s ease',
          }}
          onClick={(e) => e.stopPropagation()}
          aria-label={`${upvotes} upvotes`}
        >
          {/* Arrow up icon */}
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill={hasUpvoted ? '#ef4444' : 'none'}
            stroke={hasUpvoted ? '#ef4444' : 'currentColor'}
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            <path d="M12 19V5" />
            <path d="m5 12 7-7 7 7" />
          </svg>
          <span>{upvotes}</span>
        </button>

        {/* Comment count */}
        <button
          type="button"
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '4px',
            padding: '4px 8px',
            border: 'none',
            background: 'transparent',
            borderRadius: '4px',
            fontSize: '12px',
            color: '#6b7280',
            cursor: 'pointer',
            transition: 'background-color 0.15s ease',
          }}
          onClick={(e) => e.stopPropagation()}
          aria-label={`${commentCount} comments`}
        >
          {/* Comment icon */}
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
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
          </svg>
          <span>{commentCount}</span>
        </button>
      </footer>
    </article>
  )
}

export default PostCard
