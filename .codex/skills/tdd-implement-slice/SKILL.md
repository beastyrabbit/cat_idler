---
name: tdd-implement-slice
description: Implement one scoped feature using RED->GREEN->REFACTOR and keep edits constrained.
tools: Bash, Read, Grep, Edit, Write
---

# TDD Implement Slice

## Goal
Deliver one scoped feature with tests and minimal blast radius.

## Process
1. RED: add or update tests to encode expected behavior.
2. Run focused tests and confirm failure.
3. GREEN: implement minimal code to pass tests.
4. REFACTOR: improve readability without changing behavior.
5. Run full required checks for touched area.
6. Summarize changed files and behavior.

## Constraints
- No unrelated refactors.
- No speculative features.
- Preserve existing patterns.
