# Frozen save compatibility code

These source libraries and `persistence.rs` are exact copies of the maintained
Rust save loader at the Unity cutover. They exist only to normalize historical
SQLite saves into the typed C# import format and regenerate the fixed research
catalog. The game, shared host, controls and persistence after import run in C#.

There is no Rust server, renderer or executable in this directory. The parent
normalizer opens its explicitly selected source read-only, copies it into an
in-memory database, and runs these historical migrations on that copy. This
avoids reimplementing years of additive SQLite migrations in a second loader.

Build and verify through `../normalize-legacy` and `../test_playable_import.py`.
The embedded historical Rust test modules are retained verbatim with their source
and are not the Unity application's test inventory. Their old fixture paths
belong to the archived Rust application.
