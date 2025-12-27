/**
 * Tests for BuildMenu Component
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { BuildMenu } from '@/components/colony/BuildMenu'
import { Id } from '@/convex/_generated/dataModel'
import * as convexReact from 'convex/react'

// Mock Convex hooks
vi.mock('convex/react', () => ({
  useQuery: vi.fn(),
}))

const mockUseQuery = vi.mocked(convexReact.useQuery)

describe('BuildMenu', () => {
  const colonyId = 'colony123' as Id<'colonies'>

  const mockColony = {
    _id: colonyId,
    name: 'Test Colony',
    leaderId: null,
    status: 'thriving' as const,
    resources: {
      food: 10,
      water: 10,
      herbs: 5,
      materials: 50,
      blessings: 0,
    },
    gridSize: 5,
    createdAt: Date.now(),
    lastTick: Date.now(),
    lastAttack: 0,
  }

  const mockBuildings: any[] = []

  beforeEach(() => {
    vi.clearAllMocks()
    // Mock useQuery to return different values based on call order
    let callCount = 0
    mockUseQuery.mockImplementation(((api: any, args: any) => {
      if (args?.colonyId === colonyId) {
        callCount++
        // First call is getColony, second is getBuildingsByColony
        if (callCount === 1) {
          return mockColony
        }
        if (callCount === 2) {
          return mockBuildings
        }
      }
      return undefined
    }) as any)
  })

  it('should display buildings heading', () => {
    const onSelectType = vi.fn()
    render(<BuildMenu colonyId={colonyId} selectedType={null} onSelectType={onSelectType} />)
    expect(screen.getByText('Buildings')).toBeInTheDocument()
  })

  it('should display materials count', () => {
    const onSelectType = vi.fn()
    render(<BuildMenu colonyId={colonyId} selectedType={null} onSelectType={onSelectType} />)
    expect(screen.getByText(/Materials: 50/)).toBeInTheDocument()
  })

  it('should display building type buttons', () => {
    const onSelectType = vi.fn()
    render(<BuildMenu colonyId={colonyId} selectedType={null} onSelectType={onSelectType} />)
    expect(screen.getByText(/Food Storage/i)).toBeInTheDocument()
    expect(screen.getByText(/Water Bowl/i)).toBeInTheDocument()
  })

  it('should call onSelectType when building button clicked', async () => {
    const user = userEvent.setup()
    const onSelectType = vi.fn()
    render(<BuildMenu colonyId={colonyId} selectedType={null} onSelectType={onSelectType} />)
    
    const foodStorageButton = screen.getByText(/Food Storage/i)
    await user.click(foodStorageButton)
    
    expect(onSelectType).toHaveBeenCalledWith('food_storage')
  })

  it('should display selected building type', () => {
    const onSelectType = vi.fn()
    render(<BuildMenu colonyId={colonyId} selectedType="food_storage" onSelectType={onSelectType} />)
    expect(screen.getByText(/Click on the colony grid to place/i)).toBeInTheDocument()
  })

  it('should display "No buildings yet" when buildings array is empty', () => {
    const onSelectType = vi.fn()
    render(<BuildMenu colonyId={colonyId} selectedType={null} onSelectType={onSelectType} />)
    expect(screen.getByText('No buildings yet')).toBeInTheDocument()
  })

  it('should disable buttons when cannot afford', () => {
    const onSelectType = vi.fn()
    let callCount = 0
    mockUseQuery.mockImplementation(((api: any, args: any) => {
      if (args?.colonyId === colonyId) {
        callCount++
        if (callCount === 1) {
          return { ...mockColony, resources: { ...mockColony.resources, materials: 0 } }
        }
        if (callCount === 2) {
          return mockBuildings
        }
      }
      return undefined
    }) as any)
    
    render(<BuildMenu colonyId={colonyId} selectedType={null} onSelectType={onSelectType} />)
    const buttons = screen.getAllByRole('button')
    // Building buttons should be disabled (except cancel if selected)
    const buildingButtons = buttons.filter(btn => 
      btn.textContent?.includes('Storage') || 
      btn.textContent?.includes('Bowl')
    )
    buildingButtons.forEach(btn => {
      expect(btn).toBeDisabled()
    })
  })
})



