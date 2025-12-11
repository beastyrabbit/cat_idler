/**
 * Test Setup File
 *
 * This file runs before all tests.
 * Configure global test utilities here.
 */

import '@testing-library/jest-dom'

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

