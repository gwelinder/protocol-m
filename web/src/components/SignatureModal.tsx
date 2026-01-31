'use client'

import React, { useState, useEffect } from 'react'
import { copyToClipboard, truncateDid } from '@/lib/utils'

/** Signature envelope structure matching the server-side SignatureEnvelopeV1 */
export interface SignatureEnvelope {
  '@version': string
  type: string
  algo: string
  did: string
  hash: {
    algo: string
    value: string
  }
  artifact?: {
    name?: string
    size?: number
  }
  timestamp: string
  signature: string
  metadata?: Record<string, unknown>
}

export interface SignatureModalProps {
  /** Whether the modal is open */
  isOpen: boolean
  /** Called when the modal should close */
  onClose: () => void
  /** The signature envelope to display */
  envelope: SignatureEnvelope | null
}

/**
 * Simple JSON syntax highlighter that returns styled spans.
 * Highlights strings, numbers, booleans, null, and property keys.
 */
function highlightJson(json: string): React.ReactElement {
  // Split into tokens while preserving structure
  const lines = json.split('\n')

  return (
    <>
      {lines.map((line, lineIndex) => (
        <div key={lineIndex} style={{ minHeight: '1.4em' }}>
          {highlightLine(line)}
        </div>
      ))}
    </>
  )
}

function highlightLine(line: string): React.ReactElement {
  // Match patterns: keys, strings, numbers, booleans, null
  const parts: React.ReactElement[] = []
  let remaining = line
  let keyIndex = 0

  // Handle indentation
  const indentMatch = remaining.match(/^(\s*)/)
  if (indentMatch && indentMatch[1]) {
    parts.push(<span key={keyIndex++}>{indentMatch[1]}</span>)
    remaining = remaining.slice(indentMatch[1].length)
  }

  // Process tokens
  while (remaining.length > 0) {
    // Property key: "key":
    const keyMatch = remaining.match(/^("(?:[^"\\]|\\.)*")\s*:/)
    if (keyMatch) {
      parts.push(
        <span key={keyIndex++} style={{ color: '#9333ea' }}>
          {keyMatch[1]}
        </span>
      )
      parts.push(<span key={keyIndex++}>: </span>)
      remaining = remaining.slice(keyMatch[0].length)
      continue
    }

    // String value
    const stringMatch = remaining.match(/^("(?:[^"\\]|\\.)*")/)
    if (stringMatch) {
      parts.push(
        <span key={keyIndex++} style={{ color: '#059669' }}>
          {stringMatch[1]}
        </span>
      )
      remaining = remaining.slice(stringMatch[1].length)
      continue
    }

    // Number
    const numberMatch = remaining.match(/^(-?\d+\.?\d*(?:[eE][+-]?\d+)?)/)
    if (numberMatch) {
      parts.push(
        <span key={keyIndex++} style={{ color: '#2563eb' }}>
          {numberMatch[1]}
        </span>
      )
      remaining = remaining.slice(numberMatch[1].length)
      continue
    }

    // Boolean or null
    const literalMatch = remaining.match(/^(true|false|null)/)
    if (literalMatch) {
      parts.push(
        <span key={keyIndex++} style={{ color: '#dc2626' }}>
          {literalMatch[1]}
        </span>
      )
      remaining = remaining.slice(literalMatch[1].length)
      continue
    }

    // Other characters (braces, brackets, commas, etc.)
    parts.push(<span key={keyIndex++}>{remaining[0]}</span>)
    remaining = remaining.slice(1)
  }

  return <>{parts}</>
}

/**
 * SignatureModal component displays the full signature envelope JSON
 * with syntax highlighting and a copy button.
 *
 * Opens when clicking on a VerifiedBadge.
 */
export function SignatureModal({ isOpen, onClose, envelope }: SignatureModalProps) {
  const [copied, setCopied] = useState(false)

  // Reset copied state when modal opens/closes
  useEffect(() => {
    if (!isOpen) {
      setCopied(false)
    }
  }, [isOpen])

  // Handle escape key
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        onClose()
      }
    }
    document.addEventListener('keydown', handleEscape)
    return () => document.removeEventListener('keydown', handleEscape)
  }, [isOpen, onClose])

  if (!isOpen || !envelope) {
    return null
  }

  const jsonString = JSON.stringify(envelope, null, 2)

  const handleCopyJson = async () => {
    const success = await copyToClipboard(jsonString)
    if (success) {
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }
  }

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose()
    }
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
        zIndex: 9999,
        padding: '20px',
      }}
      onClick={handleBackdropClick}
      role="dialog"
      aria-modal="true"
      aria-labelledby="signature-modal-title"
    >
      <div
        style={{
          backgroundColor: '#ffffff',
          borderRadius: '12px',
          maxWidth: '600px',
          width: '100%',
          maxHeight: '80vh',
          display: 'flex',
          flexDirection: 'column',
          boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.25)',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div
          style={{
            padding: '16px 20px',
            borderBottom: '1px solid #e5e7eb',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <h2
            id="signature-modal-title"
            style={{
              margin: 0,
              fontSize: '16px',
              fontWeight: 600,
              color: '#111827',
            }}
          >
            Signature Envelope
          </h2>
          <button
            type="button"
            onClick={onClose}
            style={{
              padding: '4px',
              border: 'none',
              background: 'transparent',
              cursor: 'pointer',
              borderRadius: '4px',
              color: '#6b7280',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
            aria-label="Close modal"
          >
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        {/* Summary fields */}
        <div
          style={{
            padding: '16px 20px',
            borderBottom: '1px solid #e5e7eb',
            display: 'grid',
            gap: '12px',
          }}
        >
          <SummaryField label="DID" value={envelope.did} truncate />
          <SummaryField
            label="Hash"
            value={`${envelope.hash.algo}:${envelope.hash.value}`}
            truncate
          />
          <SummaryField label="Timestamp" value={envelope.timestamp} />
          <SummaryField
            label="Signature"
            value={envelope.signature}
            truncate
          />
        </div>

        {/* JSON content */}
        <div
          style={{
            flex: 1,
            overflow: 'auto',
            padding: '16px 20px',
          }}
        >
          <pre
            style={{
              margin: 0,
              padding: '12px',
              backgroundColor: '#f9fafb',
              borderRadius: '8px',
              fontSize: '12px',
              fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
              lineHeight: 1.4,
              overflow: 'auto',
              color: '#374151',
            }}
          >
            {highlightJson(jsonString)}
          </pre>
        </div>

        {/* Footer with copy button */}
        <div
          style={{
            padding: '12px 20px',
            borderTop: '1px solid #e5e7eb',
            display: 'flex',
            justifyContent: 'flex-end',
          }}
        >
          <button
            type="button"
            onClick={handleCopyJson}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              padding: '8px 16px',
              backgroundColor: copied ? '#dcfce7' : '#f3f4f6',
              border: 'none',
              borderRadius: '6px',
              fontSize: '13px',
              fontWeight: 500,
              color: copied ? '#16a34a' : '#374151',
              cursor: 'pointer',
              transition: 'background-color 0.15s ease',
            }}
          >
            {copied ? (
              <>
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <polyline points="20 6 9 17 4 12" />
                </svg>
                Copied!
              </>
            ) : (
              <>
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
                </svg>
                Copy JSON
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  )
}

interface SummaryFieldProps {
  label: string
  value: string
  truncate?: boolean
}

function SummaryField({ label, value, truncate }: SummaryFieldProps) {
  const displayValue = truncate && value.length > 50
    ? `${value.slice(0, 25)}...${value.slice(-22)}`
    : value

  return (
    <div>
      <div
        style={{
          fontSize: '11px',
          fontWeight: 500,
          color: '#6b7280',
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
          marginBottom: '4px',
        }}
      >
        {label}
      </div>
      <div
        style={{
          fontSize: '13px',
          fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
          color: '#111827',
          wordBreak: 'break-all',
        }}
        title={truncate ? value : undefined}
      >
        {displayValue}
      </div>
    </div>
  )
}

export default SignatureModal
