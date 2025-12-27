/**
 * Tests for TaskQueue Component
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { TaskQueue } from '@/components/colony/TaskQueue'
import { createMockCat } from '@/tests/factories/cat'
import type { Task } from '@/types/game'
import { Id } from '@/convex/_generated/dataModel'
import * as convexReact from 'convex/react'

// Mock Convex hooks
vi.mock('convex/react', () => ({
  useQuery: vi.fn(),
}))

const mockUseQuery = vi.mocked(convexReact.useQuery)

describe('TaskQueue', () => {
  const colonyId = 'colony123' as Id<'colonies'>
  const mockTasks: Task[] = [
    {
      _id: 'task1' as Id<'tasks'>,
      colonyId,
      type: 'hunt',
      priority: 1,
      assignedCatId: 'cat1' as Id<'cats'>,
      assignmentCountdown: 0,
      isOptimalAssignment: true,
      progress: 50,
      createdAt: Date.now(),
    },
    {
      _id: 'task2' as Id<'tasks'>,
      colonyId,
      type: 'gather_herbs',
      priority: 2,
      assignedCatId: null,
      assignmentCountdown: 5,
      isOptimalAssignment: false,
      progress: 0,
      createdAt: Date.now(),
    },
  ]

  const mockCats = [
    createMockCat({ _id: 'cat1' as Id<'cats'>, name: 'Whiskers' }),
  ]

  beforeEach(() => {
    vi.clearAllMocks()
    mockUseQuery.mockImplementation(((api: any, args: any) => {
      if (args?.colonyId === colonyId) {
        return { _id: colonyId, name: 'Test Colony' }
      }
      return undefined
    }) as any)
  })

  it('should display task queue heading', () => {
    render(<TaskQueue colonyId={colonyId} tasks={mockTasks} cats={mockCats} />)
    expect(screen.getByText('Task Queue')).toBeInTheDocument()
  })

  it('should display "No tasks" when tasks array is empty', () => {
    render(<TaskQueue colonyId={colonyId} tasks={[]} cats={[]} />)
    expect(screen.getByText('No tasks')).toBeInTheDocument()
  })

  it('should display task icons and types', () => {
    render(<TaskQueue colonyId={colonyId} tasks={mockTasks} cats={mockCats} />)
    expect(screen.getByText('hunt')).toBeInTheDocument()
    expect(screen.getByText('gather herbs')).toBeInTheDocument()
  })

  it('should display assigned cat name', () => {
    render(<TaskQueue colonyId={colonyId} tasks={mockTasks} cats={mockCats} />)
    expect(screen.getByText(/Whiskers/)).toBeInTheDocument()
  })

  it('should display pending status for unassigned tasks', () => {
    render(<TaskQueue colonyId={colonyId} tasks={mockTasks} cats={mockCats} />)
    expect(screen.getByText(/Pending/)).toBeInTheDocument()
  })

  it('should display progress percentage', () => {
    render(<TaskQueue colonyId={colonyId} tasks={mockTasks} cats={mockCats} />)
    expect(screen.getByText(/50%/)).toBeInTheDocument()
  })
})



