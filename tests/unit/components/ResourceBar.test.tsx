/**
 * Tests for ResourceBar Component
 *
 * TASK: UI-002
 */

import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ResourceBar } from '@/components/ui/ResourceBar'

describe('ResourceBar', () => {
  it('should display correct percentage progress', () => {
    const { container } = render(<ResourceBar value={50} max={100} />)
    const indicator = container.querySelector('[data-testid="resource-indicator"]')
    expect(indicator).toHaveStyle({ transform: 'translateX(-50%)' })
  })

  it('should show label when showLabel is true', () => {
    render(<ResourceBar value={50} max={100} showLabel label="Food" />)
    expect(screen.getByText('Food')).toBeInTheDocument()
    expect(screen.getByText('50/100')).toBeInTheDocument()
  })

  it('should use green color by default', () => {
    const { container } = render(<ResourceBar value={50} max={100} />)
    const indicator = container.querySelector('[data-testid="resource-indicator"]')
    expect(indicator).toHaveClass('bg-green-500')
  })

  it('should use specified color', () => {
    const { container } = render(<ResourceBar value={50} max={100} color="red" />)
    const indicator = container.querySelector('[data-testid="resource-indicator"]')
    expect(indicator).toHaveClass('bg-red-500')
  })

  it('should cap at 100% width', () => {
    const { container } = render(<ResourceBar value={150} max={100} />)
    const indicator = container.querySelector('[data-testid="resource-indicator"]')
    expect(indicator).toHaveStyle({ transform: 'translateX(-0%)' })
  })

  it('should handle zero value', () => {
    const { container } = render(<ResourceBar value={0} max={100} />)
    const indicator = container.querySelector('[data-testid="resource-indicator"]')
    expect(indicator).toHaveStyle({ transform: 'translateX(-100%)' })
  })
})



