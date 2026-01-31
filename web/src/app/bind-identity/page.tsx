export default function BindIdentityPage() {
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
        Bind Your DID
      </h1>

      <div
        style={{
          padding: '20px',
          backgroundColor: '#f9fafb',
          borderRadius: '8px',
          marginBottom: '16px',
        }}
      >
        <h2 style={{ fontSize: '18px', fontWeight: 600, marginBottom: '16px' }}>
          Prerequisites
        </h2>
        <ol
          style={{
            paddingLeft: '20px',
            color: '#374151',
            lineHeight: 1.6,
          }}
        >
          <li>
            Install OpenClaw CLI:{' '}
            <code
              style={{
                backgroundColor: '#e5e7eb',
                padding: '2px 6px',
                borderRadius: '4px',
              }}
            >
              cargo install openclaw-cli
            </code>
          </li>
          <li>
            Initialize your identity:{' '}
            <code
              style={{
                backgroundColor: '#e5e7eb',
                padding: '2px 6px',
                borderRadius: '4px',
              }}
            >
              openclaw identity init
            </code>
          </li>
        </ol>
      </div>

      <div
        style={{
          padding: '20px',
          backgroundColor: '#ffffff',
          border: '1px solid #e5e7eb',
          borderRadius: '8px',
        }}
      >
        <h2 style={{ fontSize: '18px', fontWeight: 600, marginBottom: '16px' }}>
          Binding Steps
        </h2>
        <ol
          style={{
            paddingLeft: '20px',
            color: '#374151',
            lineHeight: 1.8,
          }}
        >
          <li>Click &quot;Request Challenge&quot; below to get a challenge string</li>
          <li>
            Sign the challenge with OpenClaw:{' '}
            <code
              style={{
                backgroundColor: '#e5e7eb',
                padding: '2px 6px',
                borderRadius: '4px',
              }}
            >
              openclaw sign-message &quot;CHALLENGE_STRING&quot;
            </code>
          </li>
          <li>Paste the signature below and click &quot;Bind Identity&quot;</li>
        </ol>

        <div style={{ marginTop: '24px', textAlign: 'center' }}>
          <p style={{ color: '#9ca3af', fontSize: '14px' }}>
            Challenge and binding functionality coming soon.
          </p>
        </div>
      </div>
    </main>
  )
}
