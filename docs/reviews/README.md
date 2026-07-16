# Comprehensive Review — Idle Cat Forest

Date: 2026-07-16
Method: docs read in full, then verified against the code in `crates/` by five parallel review
passes. Numeric counts and load-bearing claims were traced to source and spot-checked rather than
trusted from `docs/IMPLEMENTATION_AUDIT.md`. The quality gates were run. The native client was
launched live against a real server on a GPU.

## The four reviews

| Document | Covers | Verdict |
| --- | --- | --- |
| [`FEATURE_REVIEW.md`](FEATURE_REVIEW.md) | Every doc feature claim vs code | High parity — nothing missing; stale recipe count |
| [`CODE_REVIEW.md`](CODE_REVIEW.md) | cat-sim, cat-server, cat-protocol, cat-client | Sound architecture; edge-case DoS + a tick-killing panic |
| [`UI_UX_REVIEW.md`](UI_UX_REVIEW.md) | Client usability + live run | Playable and readable; weak first-run/status legibility |
| [`BEST_PRACTICES_REVIEW.md`](BEST_PRACTICES_REVIEW.md) | Gates, hygiene, CI, supply chain | Strong baseline; workspace/toolchain/CI hardening identified |

All dispositions and verification evidence after implementation are maintained in
[`RESOLUTION.md`](RESOLUTION.md). The finding prose below is the pre-fix observation, not the
current backlog.

## Overall assessment

This is a mature, unusually well-documented codebase. Feature parity between the docs and the
code is high — every claimed system is genuinely implemented and wired, the determinism
discipline the project depends on is real and enforced, all quality gates pass (1,680 tests,
0 clippy warnings, formatted), and the concurrency/persistence/authorization design in the server
is careful and correct. The problems that matter are concentrated at two edges: **the untrusted
network input surface** (a reachable panic and DoS vectors) and **wire forward-compatibility**
(a newer server can blank older clients). Neither is architectural; both are fixable without
redesign.

## Original must-fix list (resolved)

1. **`ProductionQueueEdit::Move` panic** (CODE C1) — one client action panics the world tick on
   the shared colony. One-line bounds check. Verified against source at `actions.rs:1491-1503`.
2. **Server DoS cluster** (CODE server H1–H3) — unbounded village creation via freely-mintable
   sessions, connection-id rate-limit key, no WS message-size cap. Needs real client-IP binding.
3. **`designate_rail` OOM** (CODE cat-sim H1) — unbounded client coords expanded before the
   length cap.
4. **Wire enum forward-compat** (CODE protocol H4 + M1) — add `#[serde(other)]` fallbacks and
   sanitize non-finite floats, or a newer server drops the entire client frame every second.
   Verified: zero `#[serde(other)]` in `cat-protocol`.

## Original should-fix list (resolved or dispositioned)

- **Persistent disconnect indicator** (UI H1) — after the first snapshot a dead connection is
  invisible; highest-ROI UX fix.
- **First-run onboarding** (UI H2) — a deep systems game with no scaffolding.
- **Glanceable cat roles + doc fix** (UI M1) — cats are not actually tinted by specialization
  despite the docs saying so.
- **Single-pass snapshot deserialization** (CODE client CM1) — before scaling world size.
- **CI supply-chain gate** (`cargo-deny`), **workspace-inherited crate metadata**, and
  **MSRV/toolchain alignment** (BEST_PRACTICES M1–M3).

## Documentation corrections

- **Recipe count 104 → 108** in `CLAUDE.md:146,190` and `README.md:114,246` (code asserts 108).
- **"~40 phases" / "~40 modules"** undercounts (53 phases, ~60 modules).
- **"cats colored by specialization"** in `CLAUDE.md`/`GAME_VISION.md` — the client tints by
  facing direction and uses hats for 4 roles; it does not tint by specialization.
- Add the omitted cat-sim modules and the `shared_world_tiles` table to CLAUDE.md's maps.

## What the project does right

Determinism is airtight where it counts (golden-tested LCG, twin-tested subsystem seed forks,
BTree-based decision containers). The server holds no lock across `.await` or disk I/O, sheds
load on slow clients, is fully transactional with tested rollback, uses only parameterized SQL,
and gates authorization defense-in-depth through an exhaustive match. `#![forbid(unsafe_code)]`
holds in both cat-sim and cat-protocol; there is zero `unsafe` in the workspace. The client
reconciles entities in place with aggressive change-gating and renders correctly live. Secret
hygiene, git hooks, and CI all match what the docs claim.
