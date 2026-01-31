/**
 * Truncates a DID for display, showing the prefix and suffix with ellipsis in between.
 * Example: did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw -> did:key:z6Mk...Wp
 */
export function truncateDid(did: string, prefixLength = 12, suffixLength = 2): string {
  if (did.length <= prefixLength + suffixLength + 3) {
    return did
  }
  return `${did.slice(0, prefixLength)}...${did.slice(-suffixLength)}`
}

/**
 * Formats a timestamp for display.
 */
export function formatTimestamp(date: Date | string): string {
  const d = typeof date === 'string' ? new Date(date) : date
  return d.toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}

/**
 * Copies text to the clipboard.
 * Returns true if successful, false otherwise.
 */
export async function copyToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text)
    return true
  } catch {
    // Fallback for older browsers
    const textArea = document.createElement('textarea')
    textArea.value = text
    textArea.style.position = 'fixed'
    textArea.style.left = '-999999px'
    document.body.appendChild(textArea)
    textArea.select()
    try {
      document.execCommand('copy')
      return true
    } catch {
      return false
    } finally {
      document.body.removeChild(textArea)
    }
  }
}
