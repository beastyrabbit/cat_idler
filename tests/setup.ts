/**
 * Test Setup File
 *
 * This file runs before all tests.
 * Configure global test utilities here.
 */

import '@testing-library/jest-dom'
import { beforeAll } from 'vitest'

// Ensure jsdom is properly set up
beforeAll(() => {
  // Verify document exists
  if (typeof document === 'undefined') {
    throw new Error('jsdom environment not properly initialized')
  }
})

// Add any global test setup here
// For example, mock window.matchMedia for component tests:

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => {},
  }),
})

// Mock ResizeObserver
global.ResizeObserver = class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

// Ensure HTMLElement is available
if (typeof HTMLElement === 'undefined') {
  // This should not happen with jsdom, but just in case
  console.warn('HTMLElement is not defined')
}


