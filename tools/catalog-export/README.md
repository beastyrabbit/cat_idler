# Catalog export (provenance only)

The game's buildings, recipes and studies are authored C# in
`unity/Assets/Forest/Simulation/Catalog.Generated.cs` (demo decision D5 in
`docs/DEMO.md`). Edit that file by hand. Its name is historical; nothing generates it
any more.

- Classic numbers live in `Catalog.Generated.cs`. Classic worlds, the server and the
  mechanics scenarios read them unchanged.
- Demo-only numbers (recipe speed-ups, conversions, the curated study slice, merged
  stewardship studies, founding service effects) live in
  `unity/Assets/Forest/Simulation/DemoContent.cs`. Never put a demo change into the
  classic tables.
- Text (names and descriptions) is global: the same words show in classic and demo
  worlds, so describe effects without numbers that differ between them.
- Ids are stable. Saves store study and recipe ids; renaming or removing one needs a
  save migration.

The scenarios check the catalog by invariants rather than counts (`catalog.*`, the
`*_effect.*` mechanisms and `demo_content.*`):

```sh
dotnet run --project tools/scenarios -c Release -- --jobs=0 --filter=catalog
dotnet run --project tools/scenarios -c Release -- --jobs=0 --filter=effect
dotnet run --project tools/scenarios -c Release -- --jobs=0 --filter=demo_content
```

## What this folder still does

`generate.py` reproduces the catalog the frozen legacy Rust implies, so you can see
every authored change against it. It reads `catalog.json` (exported by the small Rust
program in `src/main.rs`) and two frozen Rust sources under
`tools/save-import/legacy/cat-sim/src`. Those Rust sources stay frozen and read-only.

## Diff the authored catalog against Rust

From the repository root, on a clean working tree:

```sh
# Optional: refresh catalog.json from the frozen Rust (needs a Rust toolchain).
cargo run --manifest-path tools/catalog-export/Cargo.toml --quiet > tools/catalog-export/catalog.json

# Overwrite the authored file with the Rust provenance export and look at the difference.
python3 tools/catalog-export/generate.py
git diff -- unity/Assets/Forest/Simulation/Catalog.Generated.cs

# Always put the authored file back. Never commit the export.
git restore unity/Assets/Forest/Simulation/Catalog.Generated.cs
```

The diff lists, line by line, every recipe or study whose authored data differs from
Rust: the header, rewritten names and descriptions, and any removed payload (for
example the sawmill's never-read `unlock_job gather_logs`). Mechanics differences
should be rare and each one deliberate; record why in the commit that makes it.

If `catalog.json` changed, `git diff -- tools/catalog-export/catalog.json` shows what
the Rust side moved. Commit a refreshed `catalog.json` only when you mean to update the
provenance record.

`generate.py` still asserts the legacy counts (108 recipes, 487 studies). Those are
properties of the frozen Rust export, not of the game; the authored catalog may differ.

`Smoke.cs` and `Smoke.csproj` are an old harness outside the solution. They pin legacy
counts and are not part of any check.
