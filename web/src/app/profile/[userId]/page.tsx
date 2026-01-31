import { ProfileIdentities } from '@/components/ProfileIdentities'

interface ProfilePageProps {
  params: Promise<{
    userId: string
  }>
}

// Example DIDs for demonstration
const MOCK_DIDS = [
  {
    did: 'did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw',
    createdAt: '2026-01-15T10:30:00Z',
  },
  {
    did: 'did:key:z6MkhVTX9BF3NGYX6cc7jWpbNnR7cAjH8LUffabZP8Qu4ysC',
    createdAt: '2026-01-20T14:45:00Z',
  },
]

export default async function ProfilePage({ params }: ProfilePageProps) {
  const { userId } = await params

  // In production, fetch DIDs from the API:
  // const response = await fetch(`/api/v1/profile/${userId}/dids`)
  // const data = await response.json()
  // const dids = data.dids

  // For now, use mock data
  const dids = MOCK_DIDS

  return (
    <main
      style={{
        maxWidth: '600px',
        margin: '0 auto',
        padding: '24px',
      }}
    >
      <h1
        style={{
          fontSize: '24px',
          fontWeight: 700,
          marginBottom: '24px',
        }}
      >
        Profile
      </h1>

      <div
        style={{
          padding: '16px',
          backgroundColor: '#f9fafb',
          borderRadius: '8px',
          marginBottom: '16px',
        }}
      >
        <p style={{ margin: 0, color: '#6b7280', fontSize: '14px' }}>
          User ID: {userId}
        </p>
      </div>

      <ProfileIdentities
        dids={dids}
        maxVisible={5}
        bindInstructionsUrl="/bind-identity"
      />
    </main>
  )
}
