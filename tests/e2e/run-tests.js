#!/usr/bin/env node

/**
 * Selenium E2E Test Runner
 * 
 * Runs all E2E tests using Selenium WebDriver.
 * 
 * Usage:
 *   bun run test:e2e          # Headless
 *   bun run test:e2e:headed   # With browser visible
 */

import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const isHeaded = process.env.HEADED === 'true';

// Check if server is already running, if not start it
let devServer = null;
let serverStartedByUs = false;

// First check if server is already running
const checkExistingServer = async () => {
  try {
    const response = await fetch('http://localhost:3000');
    if (response.ok) {
      serverPort = 3000;
      serverReady = true;
      console.log('✓ Using existing dev server on port 3000');
      runTests();
      return true;
    }
  } catch (e) {
    try {
      const response = await fetch('http://localhost:3001');
      if (response.ok) {
        serverPort = 3001;
        serverReady = true;
        console.log('✓ Using existing dev server on port 3001');
        runTests();
        return true;
      }
    } catch (e2) {
      // No server running, start one
    }
  }
  return false;
};

// Check for existing server first
const hasExistingServer = await checkExistingServer();

if (!hasExistingServer) {
  // Start Next.js dev server
  console.log('Starting Next.js dev server...');
  devServer = spawn('bun', ['run', 'dev'], {
    cwd: join(__dirname, '../..'),
    stdio: 'inherit',
    shell: true,
  });
  serverStartedByUs = true;
}

// Wait for server to be ready
let serverReady = false;
const maxWait = 60000; // 60 seconds
const startTime = Date.now();

let serverPort = 3000;

const checkServer = async () => {
  // Try port 3000 first
  try {
    const response = await fetch('http://localhost:3000');
    if (response.ok) {
      serverPort = 3000;
      serverReady = true;
      console.log('✓ Dev server is ready on port 3000');
      runTests();
      return;
    }
  } catch (error) {
    // Try port 3001 if 3000 fails
    try {
      const response = await fetch('http://localhost:3001');
      if (response.ok) {
        serverPort = 3001;
        serverReady = true;
        console.log('✓ Dev server is ready on port 3001');
        runTests();
        return;
      }
    } catch (e) {
      // Continue waiting
    }
  }
  
  if (Date.now() - startTime < maxWait) {
    setTimeout(checkServer, 1000);
  } else {
    console.error('✗ Dev server failed to start');
    if (devServer && !devServer.killed) {
      devServer.kill();
    }
    process.exit(1);
  }
};

const runTests = async () => {
  console.log('\nRunning E2E tests with Selenium...\n');
  
  // Import and run test files
  const testFiles = [
    './colony-lifecycle.spec.js',
    './user-interactions.spec.js',
    './building-placement.spec.js',
    './resource-bars.spec.js',
    './navigation.spec.js',
  ];

  let passed = 0;
  let failed = 0;

  // Set port as environment variable for tests
  process.env.TEST_PORT = serverPort.toString();
  
  for (const testFile of testFiles) {
    try {
      const { default: testFn } = await import(testFile);
      await testFn(isHeaded, serverPort);
      passed++;
      console.log(`✓ ${testFile} passed\n`);
    } catch (error) {
      failed++;
      console.error(`✗ ${testFile} failed:`, error.message);
      if (error.stack) {
        console.error(error.stack);
      }
    }
  }

  console.log(`\n${'='.repeat(50)}`);
  console.log(`Tests: ${passed} passed, ${failed} failed`);
  console.log(`${'='.repeat(50)}\n`);

  // Only kill server if we started it (not if it was already running)
  if (serverStartedByUs && devServer && !devServer.killed) {
    devServer.kill();
  }
  process.exit(failed > 0 ? 1 : 0);
};

// Start checking for server (only if we started one)
if (!hasExistingServer) {
  setTimeout(checkServer, 2000);
}

// Cleanup on exit
process.on('SIGINT', () => {
  console.log('\nStopping dev server...');
  if (serverStartedByUs && devServer && !devServer.killed) {
    devServer.kill();
  }
  process.exit(0);
});

process.on('SIGTERM', () => {
  if (serverStartedByUs && devServer && !devServer.killed) {
    devServer.kill();
  }
  process.exit(0);
});



