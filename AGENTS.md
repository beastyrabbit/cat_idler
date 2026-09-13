# Idle Cat Forest agent instructions

Idle Cat Forest is a non-commercial forest colony game. Unity is the selected
engine. The active migration replaces the Rust/Bevy game with C# and Unity; see
`docs/unity/ACCEPTANCE.md` for implementation and verification status.
Historical Rust-only, TypeScript parity, flat-sprite and migration-card restrictions
no longer apply. Do not reopen the engine decision.

## Current work

- `unity/Assets/Forest/Simulation`: pure C# authoritative rules and catalogs.
- `unity/Assets/Forest/Authority`: shared authentication, projection and saves.
- `unity/Assets/Forest/Presentation`: Unity rendering, input and management UI.
- `unity/Assets/Forest/Editor`: scene, build and inspection commands.
- `server`: the C# shared-world host and real socket/persistence tests.
- `tools/scenarios`: deterministic rendering-free acceptance scenarios.
- `tools/save-import`: explicit read-only legacy conversion to a new destination.
- `source-art`: editable Blender geometry and reproducible exports.

The retired Rust/Bevy application remains in Git history. Frozen libraries under
`tools/save-import/legacy` support read-only SQLite import and catalog provenance.
The archived TypeScript game on `archive/web-game` is history.

## Gameplay and authority

Read `docs/GAME_VISION.md`, current implementation/tests and the Unity acceptance
ledger before changing behavior. Preserve intended depth and player controls,
not defects or obsolete techniques. Explain intentional changes with scenarios.

The simulation must not depend on Unity, rendering, filesystem access, wall clocks,
threads or network I/O. Inject time and seeded entropy, persist stable identities,
and keep resource ownership explicit. Unity physics and camera movement cannot
create goods, move a cat through blocked routes, or bypass village ownership.
Direct control uses the same cat, inventory, needs and world authority.

All maintained recipes, research effects and manual controls need functioning,
reachable implementations. Catalog counts alone are not acceptance. Scarce goods,
beds, work slots and transport cargo cannot be double claimed. Interruption, death,
reassignment and restart must preserve or explicitly release ownership.

## Workflow and verification

Read `docs/unity/DEVELOPMENT.md` for editor/build/run/test commands. Pin Editor and
packages in project configuration. Use Editor APIs or Pipeline to edit scenes and
imported assets. Stop Play mode before C# edits and verify compilation before
reentering. Check Pipeline's nested `failed` result, not only its command envelope.

Use Blender for authored 3D objects, retaining editable sources and scripts.
Verify Unity materials, axes, scale and both cameras. Record asset/package sources
and licenses. No purchases or paid generation without specific approval.
No external AI service is required to play or test.

Every behavior change starts with a focused failing regression. Record red,
implement, make it green, then cover the composed causal chain. Never weaken an
assertion to accommodate broken behavior. During implementation run only focused
smoke tests, combined runtime at most ten seconds. After implementation run
meaningful simulation/server and Unity EditMode/PlayMode checks, long seeded
campaigns and normal UI checks in the packaged Mac app. Report measured performance
and remaining uncertainty.

Preserve unrelated user changes. Give parallel agents explicit independent
ownership. Use patch tools and package-manager commands for dependencies. Keep
Library/Temp/Logs, credentials and build products out of Git. Apple Silicon macOS
is the required target; claim other targets only after building and testing them.

## Data and delivery

Keep legacy input read-only and import to a separate destination. Never reset or
overwrite user data to make migration pass. Migrate credentials through protected
storage separately; never print them or include them in evidence. Persist full
authoritative state, not a confidential render projection.

Commit, push, PR creation, deployment and external messages require authorization.
This migration authorizes one PR, with no merge or deployment. Before publishing,
inspect the complete outgoing diff and commit range, run a fail-closed secret scan,
complete an independent review and verify its fixes. Use the authenticated GitHub
identity for the required agent attribution note. Publish GitHub image evidence
only through verified `https://schaffa.dev` links.

Commits use an imperative scoped subject and end the body with:
`Powered by human calories and mass GPU cycles.`

The Catford Examiner newspaper remains out of scope; management and the event log
serve the current game. Global security and collaboration rules still apply.
