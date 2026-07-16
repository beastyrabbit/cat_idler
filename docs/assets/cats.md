# Cat and raider sprite manifest

This is the maintained sprite contract for the Rust/Bevy top-down client. Paths are
repository-relative; `crates/cat-client/src/lib.rs::SpriteSheets` is the runtime source of
truth.

## Runtime selection

- Colonists use `public/images/cats/cat-sheet.png`.
- Rival-cat raiders use `public/images/cats/raider-sheet.png`.
- Both sheets are `1024x64`: eight direction groups, four walk frames per group, and `32x64`
  atlas cells. Direction order is S, SW, W, NW, N, NE, E, SE.
- The client keeps each body entity across snapshots, interpolates toward the latest tile, and
  selects the facing group from movement. Idle cats use frame zero.
- The existing sheets are accepted project art for the current top-down game. Replacing them is
  not a release task.

## Role and state overlays

The client loads four `32x32` role hats and follows the interpolated body with them:

| Specialization | Sprite |
| --- | --- |
| Hunter | `public/images/cats/hat-hunter.png` |
| Architect | `public/images/cats/hat-architect.png` |
| Ritualist | `public/images/cats/hat-ritualist.png` |
| Warrior | `public/images/cats/hat-warrior.png` |

Carried resources use a small semantic glyph above the interpolated body. All twenty-eight physical
`CarryingKind` values reuse their exact tracked HUD resource PNG and tint; there is no generic
colored-square or world-prop fallback. An exhaustive identity/file test and the inspected exact
1024×768 client-owned cargo fixture verify the maintained contract. Player-priority cats receive
a gold marker, and the selected cat receives a selection marker. These are follower overlays,
not additional sheet frames.

## Static portraits and retired placeholders

`cat.png`, `cat-hunter.png`, `cat-architect.png`, and `cat-ritualist.png` are static `32x32`
portraits. They are not used for walking bodies.

The coat folders (`black/`, `calico/`, `gray-tabby/`, `orange-tabby/`, `tuxedo/`, `white/`)
contain old labeled-circle placeholders. They are retained as archive assets and must not replace
the animated runtime sheet.

## Non-cat enemies

Tracked fantasy-creature stand-ins live in `public/images/game/enemies/`. The live raider body
still uses `raider-sheet.png`; these four files are catalog choices rather than a claim that each
enemy has a distinct runtime renderer:

| Token name | Sprite | Visual source cell |
| --- | --- | --- |
| Badger | `badger.png` | Roguelike Characters `(0,0)` |
| Bear | `bear.png` | Roguelike Characters `(1,2)` |
| Fox | `fox.png` | Roguelike Characters `(1,10)` |
| Rival beast | `rival_beast.png` | Roguelike Characters `(1,3)` |

These are readable game tokens rather than literal animal drawings. The sprite review bench at
`docs/sprite-review.html` is the place to compare a replacement before changing the runtime
mapping.
