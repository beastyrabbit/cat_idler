# Best-Practices Review — engineering hygiene and quality gates

> Post-review status: findings have been dispositioned and implemented where required. See
> [`RESOLUTION.md`](RESOLUTION.md); this file preserves the pre-fix evidence.

Date: 2026-07-16
Scope: quality-gate execution (fmt/clippy/nextest, full workspace), manifest/workspace hygiene,
lint posture, git hooks and CI, secret/supply-chain hygiene, doc/process
compliance against the project's own `AGENTS.md`/`CLAUDE.md` standards. No code was modified.

## Verdict

**Strong.** All three quality gates pass cleanly on the full workspace, the test suite is large
and genuinely exercised (1,680 passed, 2 intentional skips, 0 failures), secret hygiene is good,
and hooks/CI match what the docs claim. The Medium findings are concrete hardening work.

## Quality-gate results (executed 2026-07-16)

| Command | Result | Duration | Failures |
| --- | --- | --- | --- |
| `cargo fmt --all -- --check` | **PASS** | ~2 s | none |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** | ~3–4 min cold | 0 warnings |
| `cargo nextest run --workspace` | **PASS** | 309 s run (+ compile) | **1,680 passed / 2 skipped / 0 failed** |

The 2 skips are the two intentionally `#[ignore]`d tests (a food-trajectory instrument at
`cat-sim/src/world_tick.rs:63451` and an exact-cadence release playtest at
`cat-sim/tests/labor_pressure_campaign.rs:318`, both with documented reasons). 18 slow
survival/campaign tests run long (longest 230 s) but pass. Tooling present: `cargo-nextest
0.9.140`; **not** present: `cargo-audit`, `cargo-deny`.

## Findings

### Medium

- **M1 — Workspace package metadata inheritance barely used.** Root `Cargo.toml` defines shared
  package metadata, but only `cat-dev` inherited it. The other six crates hardcoded their version
  and edition and omitted other common fields, creating drift risk on the next release.
  **Action:** inherit the shared workspace package metadata in every crate.
- **M2 — No MSRV, and a toolchain mismatch surface.** No `rust-version` in any manifest;
  `Dockerfile` pins `RUST_VERSION=1.96` while `.forgejo/workflows/quality.yaml` installs
  `--default-toolchain stable`. A green CI run can diverge from the image build.
  **Action:** add `rust-version` to `[workspace.package]`; align Dockerfile/CI toolchains.
- **M3 — No supply-chain audit gate.** Neither `cargo-audit` nor `cargo-deny` runs locally or in
  CI; the 559-crate lockfile is unaudited (brief skim found nothing known-bad, but there is no
  gate). **Action:** add `cargo-deny` for advisories, bans, and dependency-source policy.
- **M4 — Stale TypeScript/Next.js scaffolding still on disk.** The working tree still holds
  `node_modules/` (528 dirs), `.next/`, `tsconfig.tsbuildinfo`, `next-env.d.ts`, and ~14 MB of
  root-level engine-eval `*.png` screenshots. All are gitignored/untracked, so nothing is
  committed — but it contradicts the "P11 cutover removed the JS toolchain" story and clutters
  the tree. **Action:** delete the local JS leftovers.

### Low

- **L1 — `.gitignore` is still the Next.js template** (`/node_modules`, `/.next/`, `.vercel`,
  `.pnp`) with Rust/Bevy rules appended. Harmless; doc-rot signal.
- **L2 — Two `#[allow(dead_code)]`** (`cat-sim/src/terrain_gen.rs:240` `HashValue`;
  `cat-client/src/lib.rs:302` `footprint_sprite`) — both carry justifying comments; legitimate
  but cleanup candidates.
- **L3 — Two root screenshots** (`issue-1-*.png`) match no `.gitignore` rule and could be
  accidentally committed.

## Compliant practices (what the project does right)

- **All gates green** — 1,680 tests including determinism-twin pairs and 100+ game-hour survival
  proofs at harsher-than-live cadences.
- **`#![forbid(unsafe_code)]` in both `cat-sim` and `cat-protocol`** (exceeds the CLAUDE.md claim
  of cat-sim only); zero `unsafe` anywhere in the workspace.
- **Secret hygiene:** no committed secrets; the only baked value is a clearly labeled
  `DEV_FALLBACK_SESSION_SECRET` (`cat-server/src/identity.rs:9`), and the server refuses to boot
  in `NODE_ENV=production` without `SESSION_HMAC_SECRET`. `data/` and `*.db` gitignored and
  untracked.
- **Hooks match docs exactly:** `lefthook.yml` = gitleaks + `cargo fmt --check` pre-commit;
  clippy + nextest pre-push — precisely as CLAUDE.md states.
- **CI is a superset of local gates:** `.forgejo/workflows/quality.yaml` mirrors
  fmt/clippy/nextest and adds a full WASM release build + transfer-size report, restricted
  permissions (`contents: read`), and cancel-in-progress concurrency. (Its first pushed run
  remains unverified, as CLAUDE.md notes — audited statically only.)
- **Dockerfile:** multi-stage, non-root (uid/gid 10001), `--locked` builds, registry cache
  mounts, `/ready` healthcheck.
- **Disciplined lint posture:** 30 `#[allow]` total, 25 of them
  `clippy::too_many_arguments`/`type_complexity` on Bevy system fns (idiomatic); no blanket
  module-level allows.
- **Manifests:** `resolver = "3"`, edition 2024, deps managed via cargo,
  `serde`/`serde_json`/`bevy` deduped through `[workspace.dependencies]`, clean native-vs-wasm
  Bevy feature split, dev-profile tuning (`opt-level = 1`, deps at 3) for Bevy compile speed.

## Fix list (priority order)

1. **M3** — add `cargo-deny` to CI.
2. **M1** — inherit shared package metadata in all crates.
3. **M2** — add `rust-version`; align Dockerfile (1.96) with CI (stable).
4. **M4/L1/L3** — delete local JS cruft, refresh `.gitignore`, remove or ignore stray root PNGs.
