/**
 * Tests for ErrorDisplay Component
 */

import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ErrorDisplay } from '@/components/ui/ErrorDisplay'

describe('ErrorDisplay', () => {
  it('should display error message as string', () => {
    render(<ErrorDisplay error="Something went wrong" />)
    expect(screen.getByText('Something went wrong')).toBeInTheDocument()
  })

  it('should display error message from Error object', () => {
    const error = new Error('Test error')
    render(<ErrorDisplay error={error} />)
    expect(screen.getByText('Test error')).toBeInTheDocument()
  })

  it('should show retry button when onRetry provided', () => {
    const onRetry = vi.fn()
    render(<ErrorDisplay error="Error" onRetry={onRetry} />)
    const retryButton = screen.getByText('Retry')
    expect(retryButton).toBeInTheDocument()
  })

  it('should call onRetry when retry button clicked', () => {
    const onRetry = vi.fn()
    render(<ErrorDisplay error="Error" onRetry={onRetry} />)
    const retryButton = screen.getByText('Retry')
    retryButton.click()
    expect(onRetry).toHaveBeenCalledTimes(1)
  })

  it('should not show retry button when onRetry not provided', () => {
    render(<ErrorDisplay error="Error" />)
    expect(screen.queryByText('Retry')).not.toBeInTheDocument()
  })
})



