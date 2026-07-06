# Path-Wear Decay Measurements

Refs #7.

Command:

```bash
./node_modules/.bin/tsx scripts/bench-path-wear-decay.ts
```

Method:

- Synthetic SQLite/WAL databases with the `worldTiles` shape used by the game.
- Compares the previous row loop (`SELECT ... WHERE colonyId = ? AND pathWear > 0`, then per-row `UPDATE`) against the SQL-side `UPDATE ... CASE ... WHERE pathWear > 0`.
- World sizes: fresh starter world (9 chunks), ~200 chunks, ~1000 chunks.
- Fixed worn set per case, with the same branch mix as the worker: low wear, floor wear, revealed trail (`63..69`), road-grade wear (`>=70`), and `road_built`.
- Timed section excludes fixture setup and per-iteration reset. 10 warmups, 80 measured iterations, one-minute decay equivalent (`decayAmount = 1`).

Results from this worktree:

| World size   |   Tiles | Legacy loop p50 | Legacy loop p95 | SQL update p50 | SQL update p95 |
| ------------ | ------: | --------------: | --------------: | -------------: | -------------: |
| fresh        |   1,296 |         0.068ms |         0.081ms |        0.022ms |        0.027ms |
| ~200 chunks  |  28,800 |         1.059ms |         1.108ms |        0.131ms |        0.143ms |
| ~1000 chunks | 144,000 |         9.100ms |         9.248ms |        0.170ms |        0.180ms |

Notes:

- The new partial index is `worldTiles_by_colony_path_wear_nonzero` on `(colonyId, pathWear) WHERE pathWear > 0`.
- The SQL update preserves the old thresholds: built roads are skipped, `63..69` remains frozen, `>=70` decays toward `63`, and low wear floors at `1`.
- These numbers isolate path-wear decay. Full `workerTick` still has other map-scale work outside this change, especially movement walk-grid construction.
