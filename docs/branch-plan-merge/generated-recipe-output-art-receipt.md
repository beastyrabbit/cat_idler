# Canonical first-output recipe art receipt

Date: 2026-07-25

Scope: all 111 canonical `art_recipe_*` keys in the content manifest. Each
recipe has its own exact destination key/path, but its initial picture is
derived from that recipe's canonical first physical output. This is deliberate
output identity, not a generic recipe fallback.

## Exact per-recipe authority

The complete one-row-per-recipe mapping is:

`tmp/imagegen/recipe-output-art-map.tsv`

SHA-256:
`d561793b74b8363658afd4a2958a55a740129f658b4e781768891adc5be48264`

It has exactly 111 rows and four tab-separated columns:

1. canonical recipe content ID;
2. canonical first-output content ID;
3. exact already accepted first-output art path;
4. exact manifest-planned `art_recipe_*` destination path.

This TSV is part of the receipt, not a transient generation helper. It is the
full exact per-recipe key list. Its destination basenames match the 111
manifest `art_recipe_*` keys with no missing, extra, or duplicate destination.
Nineteen destinations are under `assets/planned/content/`; ninety-two are under
`assets/planned/recipes/`.

The matching exact-final checksum authority is:

`tmp/imagegen/recipe-output-art-hashes.sha256`

SHA-256:
`5c539b225a6a18679c69fe0b14a668cc972ca42ed80100c7822bd7c9188bdd0c`

## Derivation rule

For each manifest recipe:

1. preserve the exact recipe ID and `art_recipe_*` key;
2. read the ordered output list from canonical content data;
3. select output index zero;
4. resolve that output's already accepted canonical art;
5. copy those exact `16×16` pixels to the recipe's own manifest-planned path;
6. never search by category, borrow a nearby picture, or fall back to a
   generic food/tool/material icon.

All 111 destination files are byte-identical to the exact source path recorded
on their own TSV row. Repeated hashes are expected only where multiple recipes
have the same canonical first output; each still retains its own exact
`art_recipe_*` key and destination.

## First-output coverage

| Canonical first output | Recipe count |
| --- | ---: |
| Individual prepared foods other than Brew | 18 |
| `food_brew` | 5 |
| `item_armor` | 10 |
| `item_bowl` | 6 |
| `item_brick` | 1 |
| `item_furniture` | 5 |
| `item_generic_tool` | 10 |
| `item_mug` | 6 |
| `item_toy` | 5 |
| `item_treated_pelt_clothing` | 4 |
| `item_trinket` | 3 |
| `item_weapon` | 7 |
| `resource_blocks` | 2 |
| `resource_cloth` | 6 |
| `resource_flour` | 1 |
| `resource_leather` | 6 |
| `resource_lumber` | 3 |
| `resource_medicine` | 5 |
| `resource_metal` | 5 |
| `resource_planks` | 2 |
| `resource_thread` | 1 |
| **Total** | **111** |

The eighteen individual-food count covers one recipe each for Apple Porridge,
Apple Preserves, Apple Tart, Baked Apples, Dried Meat, Festival Cake, Fish
Stew, Flatbread, Grand Lair Feast, Grilled Fish, Herb-crusted Fish, Hunter's
Feast, Meat Pie, Meat Stew, Roasted Meat, Smoked Fish, Surf and Turf, and
Travel Rations.

## Verification and current integration state

- Exact destination count: 111.
- Exact manifest-key set equality: clean.
- Unique destination paths: 111.
- Missing files: zero.
- Final dimensions: 111 × `16×16` sRGBA.
- Source/destination byte-identity check: 111/111.
- Checksum rows: 111.
- Positive resolver state at receipt time: **pending for all 111 recipe keys**.

The art delivery is complete, but the resolver must add exact positive mappings
for these 111 keys before canonical manifest-key coverage can be called
runtime-complete. Until then there are zero missing recipe files and 111
resolver gaps.
