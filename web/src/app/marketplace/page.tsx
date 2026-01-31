'use client'

import { useState, useEffect } from 'react'
import { BountyCard, ClosureType, BountyStatus } from '@/components/BountyCard'

/** Bounty data from API */
interface Bounty {
  id: string
  title: string
  description: string
  reward_credits: string
  poster_did: string
  closure_type: ClosureType
  status: BountyStatus
  created_at: string
  deadline: string | null
}

/** API response for listing bounties */
interface BountiesResponse {
  bounties: Bounty[]
  total: number
}

/**
 * Mock bounties for development/demo purposes.
 * In production, this would come from GET /api/v1/bounties?status=open
 */
const MOCK_BOUNTIES: Bounty[] = [
  {
    id: '550e8400-e29b-41d4-a716-446655440001',
    title: 'Implement OAuth2 authentication flow',
    description:
      'We need a complete OAuth2 implementation supporting Google, GitHub, and custom OIDC providers. Must include refresh token handling, secure token storage, and proper PKCE flow for SPAs.',
    reward_credits: '500.00000000',
    poster_did: 'did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw',
    closure_type: 'tests',
    status: 'open',
    created_at: '2026-01-30T10:00:00Z',
    deadline: '2026-02-15T23:59:59Z',
  },
  {
    id: '550e8400-e29b-41d4-a716-446655440002',
    title: 'Code review: Smart contract audit',
    description:
      'Need experienced reviewers to audit our Solidity smart contracts for security vulnerabilities. Looking for issues like reentrancy, overflow, and access control problems.',
    reward_credits: '1250.00000000',
    poster_did: 'did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK',
    closure_type: 'quorum',
    status: 'open',
    created_at: '2026-01-29T15:30:00Z',
    deadline: '2026-02-10T23:59:59Z',
  },
  {
    id: '550e8400-e29b-41d4-a716-446655440003',
    title: 'Design system icons - AI/ML theme',
    description:
      'Create a set of 24 consistent icons for our AI/ML platform. Includes icons for models, training, inference, datasets, and more. SVG format required.',
    reward_credits: '200.00000000',
    poster_did: 'did:key:z6MkpTHR8VNs4Z5F2X1tYn3L9gQK8jKm4xZBn5JvCb9DqFRe',
    closure_type: 'requester',
    status: 'open',
    created_at: '2026-01-31T08:15:00Z',
    deadline: null,
  },
  {
    id: '550e8400-e29b-41d4-a716-446655440004',
    title: 'Fix memory leak in WebSocket handler',
    description:
      'Our production WebSocket server is experiencing memory leaks under high connection churn. Need to identify the root cause and implement a fix. Test suite included.',
    reward_credits: '350.00000000',
    poster_did: 'did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw',
    closure_type: 'tests',
    status: 'open',
    created_at: '2026-01-28T12:00:00Z',
    deadline: '2026-02-05T18:00:00Z',
  },
  {
    id: '550e8400-e29b-41d4-a716-446655440005',
    title: 'Write technical documentation for API v2',
    description:
      'Document our new API v2 endpoints including request/response schemas, authentication requirements, rate limits, and example usage. OpenAPI spec preferred.',
    reward_credits: '175.00000000',
    poster_did: 'did:key:z6MkqRYqQiSgvZQdnBytw86Qbs2ZWUkGv22od935YF4s8M7V',
    closure_type: 'requester',
    status: 'open',
    created_at: '2026-01-31T09:00:00Z',
    deadline: '2026-02-20T23:59:59Z',
  },
]

/**
 * Marketplace page displaying open bounties.
 * Bounties can be browsed by all users.
 */
export default function MarketplacePage() {
  const [bounties, setBounties] = useState<Bounty[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    async function fetchBounties() {
      try {
        // In production, fetch from API:
        // const response = await fetch('/api/v1/bounties?status=open')
        // const data: BountiesResponse = await response.json()
        // setBounties(data.bounties)

        // For now, use mock data with simulated delay
        await new Promise((resolve) => setTimeout(resolve, 500))
        setBounties(MOCK_BOUNTIES)
      } catch (err) {
        setError('Failed to load bounties. Please try again.')
        console.error('Error fetching bounties:', err)
      } finally {
        setLoading(false)
      }
    }

    fetchBounties()
  }, [])

  return (
    <main
      style={{
        maxWidth: '900px',
        margin: '0 auto',
        padding: '24px 16px',
      }}
    >
      {/* Page header */}
      <header style={{ marginBottom: '24px' }}>
        <h1
          style={{
            margin: '0 0 8px 0',
            fontSize: '24px',
            fontWeight: 700,
            color: '#111827',
          }}
        >
          Bounty Marketplace
        </h1>
        <p
          style={{
            margin: 0,
            fontSize: '14px',
            color: '#6b7280',
          }}
        >
          Browse open tasks and earn M-credits by completing bounties
        </p>
      </header>

      {/* Stats bar */}
      <div
        style={{
          display: 'flex',
          gap: '24px',
          marginBottom: '24px',
          padding: '12px 16px',
          backgroundColor: '#f9fafb',
          borderRadius: '8px',
          fontSize: '13px',
        }}
      >
        <div>
          <span style={{ color: '#6b7280' }}>Open bounties: </span>
          <span style={{ fontWeight: 600, color: '#111827' }}>{bounties.length}</span>
        </div>
        <div>
          <span style={{ color: '#6b7280' }}>Total rewards: </span>
          <span style={{ fontWeight: 600, color: '#059669' }}>
            {bounties
              .reduce((sum, b) => sum + parseFloat(b.reward_credits), 0)
              .toFixed(2)}{' '}
            M
          </span>
        </div>
      </div>

      {/* Loading state */}
      {loading && (
        <div
          style={{
            textAlign: 'center',
            padding: '48px',
            color: '#6b7280',
          }}
        >
          Loading bounties...
        </div>
      )}

      {/* Error state */}
      {error && (
        <div
          style={{
            textAlign: 'center',
            padding: '48px',
            color: '#dc2626',
            backgroundColor: '#fef2f2',
            borderRadius: '8px',
          }}
        >
          {error}
        </div>
      )}

      {/* Empty state */}
      {!loading && !error && bounties.length === 0 && (
        <div
          style={{
            textAlign: 'center',
            padding: '48px',
            color: '#6b7280',
            backgroundColor: '#f9fafb',
            borderRadius: '8px',
          }}
        >
          <p style={{ margin: '0 0 8px 0', fontSize: '16px', fontWeight: 500 }}>
            No open bounties
          </p>
          <p style={{ margin: 0, fontSize: '14px' }}>
            Check back later for new tasks
          </p>
        </div>
      )}

      {/* Bounty list */}
      {!loading && !error && bounties.length > 0 && (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '12px',
          }}
        >
          {bounties.map((bounty) => (
            <BountyCard
              key={bounty.id}
              id={bounty.id}
              title={bounty.title}
              description={bounty.description}
              rewardCredits={bounty.reward_credits}
              posterDid={bounty.poster_did}
              closureType={bounty.closure_type}
              status={bounty.status}
              createdAt={bounty.created_at}
              deadline={bounty.deadline}
              onClick={() => {
                // TODO: Navigate to bounty details page
                console.log('View bounty:', bounty.id)
              }}
            />
          ))}
        </div>
      )}
    </main>
  )
}
