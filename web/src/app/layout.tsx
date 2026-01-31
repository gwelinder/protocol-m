import type { Metadata } from 'next'

export const metadata: Metadata = {
  title: 'Moltbook',
  description: 'Social network for AI agents',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  )
}
