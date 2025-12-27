/**
 * Tests for EventLog Component
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { EventLog } from '@/components/colony/EventLog'
import { Id } from '@/convex/_generated/dataModel'
import * as convexReact from 'convex/react'

// Mock Convex hooks
vi.mock('convex/react', () => ({
  useQuery: vi.fn(),
}))

const mockUseQuery = vi.mocked(convexReact.useQuery)

describe('EventLog', () => {
  const colonyId = 'colony123' as Id<'colonies'>

  const mockEvents = [
    {
      _id: 'event1' as Id<'events'>,
      colonyId,
      timestamp: Date.now() - 60000, // 1 minute ago
      type: 'birth' as const,
      message: 'Kitten born!',
      involvedCatIds: ['cat1' as Id<'cats'>],
      metadata: {},
    },
    {
      _id: 'event2' as Id<'events'>,
      colonyId,
      timestamp: Date.now() - 3600000, // 1 hour ago
      type: 'task_complete' as const,
      message: 'Task completed',
      involvedCatIds: [],
      metadata: {},
    },
  ]

  beforeEach(() => {
    vi.clearAllMocks()
    // Mock Date.now for consistent time formatting
    vi.spyOn(Date, 'now').mockReturnValue(Date.now())
  })

  it('should display event log heading', () => {
    mockUseQuery.mockReturnValue(mockEvents)
    render(<EventLog colonyId={colonyId} />)
    expect(screen.getByText('Event Log')).toBeInTheDocument()
  })

  it('should display "No events yet" when events array is empty', () => {
    mockUseQuery.mockReturnValue([])
    render(<EventLog colonyId={colonyId} />)
    expect(screen.getByText('No events yet')).toBeInTheDocument()
  })

  it('should display loading state when events is undefined', () => {
    mockUseQuery.mockReturnValue(undefined)
    render(<EventLog colonyId={colonyId} />)
    expect(screen.getByText('Loading events...')).toBeInTheDocument()
  })

  it('should display event messages', () => {
    mockUseQuery.mockReturnValue(mockEvents)
    render(<EventLog colonyId={colonyId} />)
    expect(screen.getByText('Kitten born!')).toBeInTheDocument()
    expect(screen.getByText('Task completed')).toBeInTheDocument()
  })

  it('should display event icons', () => {
    mockUseQuery.mockReturnValue(mockEvents)
    const { container } = render(<EventLog colonyId={colonyId} />)
    // Check for emoji icons (birth = 👶)
    const pageText = container.textContent || ''
    expect(pageText).toContain('👶')
  })
})



