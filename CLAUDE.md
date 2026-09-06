# Idle Cat Forest

Read [AGENTS.md](AGENTS.md) for the current project instructions and
[docs/unity/ACCEPTANCE.md](docs/unity/ACCEPTANCE.md) for verified status and remaining
work. Unity and C# are the active application. The required native target is Apple
Silicon macOS.

The pure simulation lives in `unity/Assets/Forest/Simulation`; shared authority,
identity and saves live in `unity/Assets/Forest/Authority`. The Unity presentation
and editor commands are alongside them. `server` hosts the same rules over signed
WebSocket connections. `source-art` contains editable Blender assets and exports.

Use [README.md](README.md) and
[docs/unity/DEVELOPMENT.md](docs/unity/DEVELOPMENT.md) for setup, run, build and test
commands. Architecture is maintained in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). Do not duplicate those instructions
here or use historical Bevy, Cargo, Trunk or TypeScript commands to run the game.

Rust source under `tools/save-import/legacy` is frozen compatibility code for
explicit, read-only SQLite conversion and catalog provenance. It is not another
game application. Follow [docs/unity/PERSISTENCE.md](docs/unity/PERSISTENCE.md) to
import a selected backup to a new destination. Never reset player data or expose
the separate identity credentials.
