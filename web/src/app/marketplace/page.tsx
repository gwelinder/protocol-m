'use client'

import { useState, useEffect, useMemo, useCallback } from 'react'
import { useRouter, useSearchParams } from 'next/navigation'
import { BountyCard, ClosureType, BountyStatus } from '@/components/BountyCard'
import { AcceptBountyModal, SubmissionInstructions } from '@/components/AcceptBountyModal'

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

/** Sort options */
type SortOption = 'newest' | 'reward' | 'ending'

/** Reward range filter options */
type RewardRange = 'all' | '0-100' | '100-500' | '500-1000' | '1000+'

/** Deadline filter options */
type DeadlineFilter = 'all' | 'today' | 'week' | 'month' | 'no_deadline'

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
 * Parse reward range into min/max values
 */
function parseRewardRange(range: RewardRange): { min: number; max: number | null } {
  switch (range) {
    case '0-100':
      return { min: 0, max: 100 }
    case '100-500':
      return { min: 100, max: 500 }
    case '500-1000':
      return { min: 500, max: 1000 }
    case '1000+':
      return { min: 1000, max: null }
    default:
      return { min: 0, max: null }
  }
}

/**
 * Check if bounty deadline matches filter
 */
function matchesDeadlineFilter(deadline: string | null, filter: DeadlineFilter): boolean {
  if (filter === 'all') return true
  if (filter === 'no_deadline') return deadline === null

  if (!deadline) return false

  const deadlineDate = new Date(deadline)
  const now = new Date()
  const diffMs = deadlineDate.getTime() - now.getTime()
  const diffDays = diffMs / (1000 * 60 * 60 * 24)

  switch (filter) {
    case 'today':
      return diffDays >= 0 && diffDays < 1
    case 'week':
      return diffDays >= 0 && diffDays <= 7
    case 'month':
      return diffDays >= 0 && diffDays <= 30
    default:
      return true
  }
}

/**
 * Marketplace page displaying open bounties with filters and sorting.
 * Bounties can be browsed by all users.
 */
export default function MarketplacePage() {
  const router = useRouter()
  const searchParams = useSearchParams()

  const [bounties, setBounties] = useState<Bounty[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Accept bounty modal state
  const [selectedBounty, setSelectedBounty] = useState<Bounty | null>(null)
  const [isAcceptModalOpen, setIsAcceptModalOpen] = useState(false)

  // Mock user state - in production, this would come from auth context
  // For development, we simulate a user with/without bound DID
  const [userDid, setUserDid] = useState<string | null>(null)
  const hasBoundDid = userDid !== null

  // Simulate fetching user DID status on mount
  useEffect(() => {
    // In production: fetch from /api/v1/profile/{userId}/dids
    // For demo, simulate a bound DID after a short delay
    const timer = setTimeout(() => {
      // Uncomment to simulate user WITH bound DID:
      setUserDid('did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw')
      // Keep null to simulate user WITHOUT bound DID
    }, 100)
    return () => clearTimeout(timer)
  }, [])

  // Filter state - initialized from URL params
  const [search, setSearch] = useState(searchParams.get('q') ?? '')
  const [closureType, setClosureType] = useState<ClosureType | 'all'>(
    (searchParams.get('type') as ClosureType | 'all') ?? 'all'
  )
  const [rewardRange, setRewardRange] = useState<RewardRange>(
    (searchParams.get('reward') as RewardRange) ?? 'all'
  )
  const [deadlineFilter, setDeadlineFilter] = useState<DeadlineFilter>(
    (searchParams.get('deadline') as DeadlineFilter) ?? 'all'
  )
  const [sortBy, setSortBy] = useState<SortOption>(
    (searchParams.get('sort') as SortOption) ?? 'newest'
  )

  // Update URL when filters change
  const updateUrlParams = useCallback(
    (params: Record<string, string | null>) => {
      const newParams = new URLSearchParams(searchParams.toString())

      Object.entries(params).forEach(([key, value]) => {
        if (value && value !== 'all' && value !== '') {
          newParams.set(key, value)
        } else {
          newParams.delete(key)
        }
      })

      // Keep 'newest' as default, don't add to URL
      if (newParams.get('sort') === 'newest') {
        newParams.delete('sort')
      }

      const queryString = newParams.toString()
      router.push(queryString ? `?${queryString}` : '/marketplace', { scroll: false })
    },
    [router, searchParams]
  )

  // Handle filter changes
  const handleSearchChange = useCallback(
    (value: string) => {
      setSearch(value)
      updateUrlParams({ q: value || null })
    },
    [updateUrlParams]
  )

  const handleClosureTypeChange = useCallback(
    (value: ClosureType | 'all') => {
      setClosureType(value)
      updateUrlParams({ type: value === 'all' ? null : value })
    },
    [updateUrlParams]
  )

  const handleRewardRangeChange = useCallback(
    (value: RewardRange) => {
      setRewardRange(value)
      updateUrlParams({ reward: value === 'all' ? null : value })
    },
    [updateUrlParams]
  )

  const handleDeadlineFilterChange = useCallback(
    (value: DeadlineFilter) => {
      setDeadlineFilter(value)
      updateUrlParams({ deadline: value === 'all' ? null : value })
    },
    [updateUrlParams]
  )

  const handleSortChange = useCallback(
    (value: SortOption) => {
      setSortBy(value)
      updateUrlParams({ sort: value })
    },
    [updateUrlParams]
  )

  // Clear all filters
  const clearFilters = useCallback(() => {
    setSearch('')
    setClosureType('all')
    setRewardRange('all')
    setDeadlineFilter('all')
    setSortBy('newest')
    router.push('/marketplace', { scroll: false })
  }, [router])

  // Handle accepting a bounty
  const handleAcceptBounty = useCallback((bountyId: string) => {
    const bounty = bounties.find(b => b.id === bountyId)
    if (bounty) {
      setSelectedBounty(bounty)
      setIsAcceptModalOpen(true)
    }
  }, [bounties])

  // Handle navigating to bind DID page
  const handleNavigateToBindDid = useCallback(() => {
    setIsAcceptModalOpen(false)
    router.push('/bind-identity')
  }, [router])

  // Handle confirming bounty acceptance
  const handleConfirmAccept = useCallback(async (bountyId: string): Promise<SubmissionInstructions | null> => {
    // In production, this would:
    // 1. Call POST /api/v1/bounties/{bountyId}/accept with the user's auth token
    // 2. Return the submission instructions from the response

    // For demo, simulate API call
    await new Promise(resolve => setTimeout(resolve, 800))

    // Mock response based on the bounty
    const bounty = bounties.find(b => b.id === bountyId)
    if (!bounty) return null

    // Update the bounty status in local state
    setBounties(prev => prev.map(b =>
      b.id === bountyId ? { ...b, status: 'in_progress' as BountyStatus } : b
    ))

    // Return mock submission instructions
    const mockInstructions: SubmissionInstructions = {
      endpoint: `/api/v1/bounties/${bountyId}/submit`,
      closureType: bounty.closure_type,
      requirements: {
        type: bounty.closure_type,
        description: bounty.closure_type === 'tests'
          ? 'Your submission must pass the automated test harness.'
          : bounty.closure_type === 'quorum'
          ? 'Your submission will be reviewed by multiple peer reviewers.'
          : 'Your submission will be reviewed and approved by the bounty poster.',
        evalHarnessHash: bounty.closure_type === 'tests' ? 'sha256:abc123def456...' : undefined,
        reviewerCount: bounty.closure_type === 'quorum' ? 3 : undefined,
        minReviewerReputation: bounty.closure_type === 'quorum' ? 50 : undefined,
        requiredFields: bounty.closure_type === 'tests'
          ? ['signatureEnvelope', 'executionReceipt']
          : ['signatureEnvelope']
      },
      deadline: bounty.deadline
    }

    return mockInstructions
  }, [bounties])

  // Close the accept modal
  const handleCloseAcceptModal = useCallback(() => {
    setIsAcceptModalOpen(false)
    setSelectedBounty(null)
  }, [])

  // Check if any filters are active
  const hasActiveFilters =
    search !== '' ||
    closureType !== 'all' ||
    rewardRange !== 'all' ||
    deadlineFilter !== 'all' ||
    sortBy !== 'newest'

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

  // Filter and sort bounties
  const filteredBounties = useMemo(() => {
    let result = [...bounties]

    // Search filter
    if (search) {
      const searchLower = search.toLowerCase()
      result = result.filter(
        (b) =>
          b.title.toLowerCase().includes(searchLower) ||
          b.description.toLowerCase().includes(searchLower)
      )
    }

    // Closure type filter
    if (closureType !== 'all') {
      result = result.filter((b) => b.closure_type === closureType)
    }

    // Reward range filter
    if (rewardRange !== 'all') {
      const { min, max } = parseRewardRange(rewardRange)
      result = result.filter((b) => {
        const reward = parseFloat(b.reward_credits)
        if (max === null) {
          return reward >= min
        }
        return reward >= min && reward < max
      })
    }

    // Deadline filter
    if (deadlineFilter !== 'all') {
      result = result.filter((b) => matchesDeadlineFilter(b.deadline, deadlineFilter))
    }

    // Sort
    switch (sortBy) {
      case 'newest':
        result.sort(
          (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
        )
        break
      case 'reward':
        result.sort((a, b) => parseFloat(b.reward_credits) - parseFloat(a.reward_credits))
        break
      case 'ending':
        result.sort((a, b) => {
          // Bounties with no deadline go to the end
          if (!a.deadline && !b.deadline) return 0
          if (!a.deadline) return 1
          if (!b.deadline) return -1
          return new Date(a.deadline).getTime() - new Date(b.deadline).getTime()
        })
        break
    }

    return result
  }, [bounties, search, closureType, rewardRange, deadlineFilter, sortBy])

  // Styles for filter controls
  const selectStyle: React.CSSProperties = {
    padding: '8px 12px',
    fontSize: '13px',
    border: '1px solid #e5e7eb',
    borderRadius: '6px',
    backgroundColor: '#ffffff',
    color: '#374151',
    cursor: 'pointer',
    minWidth: '120px',
  }

  const inputStyle: React.CSSProperties = {
    padding: '8px 12px',
    fontSize: '13px',
    border: '1px solid #e5e7eb',
    borderRadius: '6px',
    backgroundColor: '#ffffff',
    color: '#374151',
    flex: 1,
    minWidth: '200px',
  }

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

      {/* Search and filters */}
      <div
        style={{
          marginBottom: '16px',
          display: 'flex',
          flexDirection: 'column',
          gap: '12px',
        }}
      >
        {/* Search bar */}
        <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
          <div style={{ position: 'relative', flex: 1 }}>
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="#9ca3af"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              style={{
                position: 'absolute',
                left: '12px',
                top: '50%',
                transform: 'translateY(-50%)',
              }}
              aria-hidden="true"
            >
              <circle cx="11" cy="11" r="8" />
              <line x1="21" y1="21" x2="16.65" y2="16.65" />
            </svg>
            <input
              type="text"
              placeholder="Search bounties..."
              value={search}
              onChange={(e) => handleSearchChange(e.target.value)}
              style={{
                ...inputStyle,
                paddingLeft: '36px',
              }}
              aria-label="Search bounties by title or description"
            />
          </div>
        </div>

        {/* Filter dropdowns */}
        <div
          style={{
            display: 'flex',
            flexWrap: 'wrap',
            gap: '12px',
            alignItems: 'center',
          }}
        >
          {/* Closure type filter */}
          <select
            value={closureType}
            onChange={(e) => handleClosureTypeChange(e.target.value as ClosureType | 'all')}
            style={selectStyle}
            aria-label="Filter by closure type"
          >
            <option value="all">All Types</option>
            <option value="tests">Tests</option>
            <option value="quorum">Quorum</option>
            <option value="requester">Requester</option>
          </select>

          {/* Reward range filter */}
          <select
            value={rewardRange}
            onChange={(e) => handleRewardRangeChange(e.target.value as RewardRange)}
            style={selectStyle}
            aria-label="Filter by reward range"
          >
            <option value="all">All Rewards</option>
            <option value="0-100">0 - 100 M</option>
            <option value="100-500">100 - 500 M</option>
            <option value="500-1000">500 - 1000 M</option>
            <option value="1000+">1000+ M</option>
          </select>

          {/* Deadline filter */}
          <select
            value={deadlineFilter}
            onChange={(e) => handleDeadlineFilterChange(e.target.value as DeadlineFilter)}
            style={selectStyle}
            aria-label="Filter by deadline"
          >
            <option value="all">All Deadlines</option>
            <option value="today">Ending Today</option>
            <option value="week">This Week</option>
            <option value="month">This Month</option>
            <option value="no_deadline">No Deadline</option>
          </select>

          {/* Sort dropdown */}
          <select
            value={sortBy}
            onChange={(e) => handleSortChange(e.target.value as SortOption)}
            style={{
              ...selectStyle,
              marginLeft: 'auto',
            }}
            aria-label="Sort bounties"
          >
            <option value="newest">Newest First</option>
            <option value="reward">Highest Reward</option>
            <option value="ending">Ending Soon</option>
          </select>

          {/* Clear filters button */}
          {hasActiveFilters && (
            <button
              onClick={clearFilters}
              style={{
                padding: '8px 12px',
                fontSize: '13px',
                border: '1px solid #e5e7eb',
                borderRadius: '6px',
                backgroundColor: '#ffffff',
                color: '#6b7280',
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: '4px',
              }}
              aria-label="Clear all filters"
            >
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
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
              Clear
            </button>
          )}
        </div>
      </div>

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
          <span style={{ color: '#6b7280' }}>
            {filteredBounties.length === bounties.length ? 'Open bounties: ' : 'Showing: '}
          </span>
          <span style={{ fontWeight: 600, color: '#111827' }}>
            {filteredBounties.length}
            {filteredBounties.length !== bounties.length && (
              <span style={{ color: '#6b7280', fontWeight: 400 }}> of {bounties.length}</span>
            )}
          </span>
        </div>
        <div>
          <span style={{ color: '#6b7280' }}>Total rewards: </span>
          <span style={{ fontWeight: 600, color: '#059669' }}>
            {filteredBounties
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

      {/* Empty state - no bounties at all */}
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
          <p style={{ margin: 0, fontSize: '14px' }}>Check back later for new tasks</p>
        </div>
      )}

      {/* Empty state - no matching bounties */}
      {!loading && !error && bounties.length > 0 && filteredBounties.length === 0 && (
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
            No bounties match your filters
          </p>
          <p style={{ margin: '0 0 16px 0', fontSize: '14px' }}>
            Try adjusting your search or filters
          </p>
          <button
            onClick={clearFilters}
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
            Clear Filters
          </button>
        </div>
      )}

      {/* Bounty list */}
      {!loading && !error && filteredBounties.length > 0 && (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '12px',
          }}
        >
          {filteredBounties.map((bounty) => (
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
              onAccept={handleAcceptBounty}
            />
          ))}
        </div>
      )}

      {/* Accept Bounty Modal */}
      {selectedBounty && (
        <AcceptBountyModal
          isOpen={isAcceptModalOpen}
          onClose={handleCloseAcceptModal}
          bountyId={selectedBounty.id}
          bountyTitle={selectedBounty.title}
          rewardCredits={selectedBounty.reward_credits}
          closureType={selectedBounty.closure_type}
          hasBoundDid={hasBoundDid}
          onNavigateToBindDid={handleNavigateToBindDid}
          onConfirmAccept={handleConfirmAccept}
          userDid={userDid ?? undefined}
        />
      )}
    </main>
  )
}
