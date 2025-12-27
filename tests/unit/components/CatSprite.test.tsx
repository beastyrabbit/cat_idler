/**
 * Tests for CatSprite Component
 *
 * TASK: UI-001
 */

import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { CatSprite } from '@/components/colony/CatSprite'
import { createMockCat } from '@/tests/factories/cat'

describe('CatSprite', () => {
  it('should render cat name in fallback', () => {
    const mockCat = createMockCat({ name: 'Whiskers' })
    render(<CatSprite cat={mockCat} />)
    
    const sprite = screen.getByTestId('cat-sprite-local')
    expect(sprite).toBeInTheDocument()
    expect(screen.getByAltText(/Whiskers/i)).toBeInTheDocument()
  })

  it('should show fallback when no sprite params', () => {
    const mockCat = createMockCat({ spriteParams: null })
    render(<CatSprite cat={mockCat} />)
    
    expect(screen.getByTestId('cat-sprite-local')).toBeInTheDocument()
  })

  it('should apply size class', () => {
    const mockCat = createMockCat()
    const { container } = render(<CatSprite cat={mockCat} size="large" />)
    
    const sprite = container.querySelector('[data-testid="cat-sprite-local"]')
    expect(sprite).toHaveClass('w-32', 'h-32')
  })

  it('should be clickable when onClick provided', () => {
    const mockCat = createMockCat()
    const handleClick = vi.fn()
    render(<CatSprite cat={mockCat} onClick={handleClick} />)
    
    const sprite = screen.getByTestId('cat-sprite-local')
    sprite.click()
    expect(handleClick).toHaveBeenCalledTimes(1)
  })

  it('should apply animation class when animated', () => {
    const mockCat = createMockCat()
    const { container } = render(<CatSprite cat={mockCat} animated={true} />)
    
    const sprite = container.querySelector('[data-testid="cat-sprite-local"]')
    expect(sprite).toHaveClass('animate-cozy-bob')
  })
})



