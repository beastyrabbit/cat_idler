# Generated Lair Art Receipt

Status: production candidate asset set, integrated by canonical public art key.

This receipt records the missing ten Lair-band sprites required by P1.09,
P1.36, LAI.42, LAI.49, and LAI.68. The protected Shrine source supplied one
generic Lair sprite only, so these ten variants were generated rather than
silently stretching that single image across all public bands.

## Generation mode and reference

- Tool mode: built-in OpenAI image generation, image-edit/reference mode.
- Reference image:
  `public/images/game/sites/lair.png`.
- Final generation directory retained by the local tool:
  `/home/beasty/.config/orca/codex-runtime-home/home/generated_images/019f8a4e-ff13-7991-b8a8-b2d97232c763/`.
- Project output directory: `assets/planned/lairs/`.
- The art communicates only the public ten-level Lair band. It contains no
  text, exact level, creature roster, statistics, ecology, regeneration,
  respawn deadline, or hidden boss information.

## Prompt specification

Every final image used this common prompt contract:

> Create one isolated, top-down three-quarter pixel-art fantasy enemy lair for
> Idle Cat Forest. Match the supplied Lair reference's compact readable
> silhouette, dark-forest palette, hand-placed pixel texture, and game-world
> scale. Show no cats, creatures, UI, text, numbers, badges, exact level, or
> background scenery. Center the complete structure with generous padding on a
> flat saturated magenta chroma background. Keep the lair recognizable at
> 80×80 after nearest-neighbour reduction.

The final per-file prompt suffixes were:

1. `01–10`: a small natural burrow, sparse stones and roots, weakest and least
   threatening band.
2. `11–20`: a reinforced woodland den, more stones, roots, and a darker mouth.
3. `21–30`: a larger rocky hideout with timber reinforcement and subtle bones.
4. `31–40`: a dangerous stone cave with thorny growth and a deeper entrance.
5. `41–50`: a fortified predator den with heavy rock, stakes, and worn trophies.
6. `51–60`: an ominous ancient cavern with denser fortification and faint
   violet corruption.
7. `61–70`: a mystic lair with runic stone, dark crystal, and controlled violet
   energy.
8. `71–80`: a powerful monster stronghold with larger crystals, runes, and
   obsidian structure.
9. `81–90`: an imposing late-game fortress-cavern with severe obsidian and
   violet magical corruption.
10. `91–100`: the strongest monumental ancient lair, a restrained
    obsidian/violet end-game landmark without depicting or identifying a boss.

The first six final calls were produced as adjacent-band refinements of the
same reference and the last four as deliberately stronger late-game
refinements. Broad-range alternatives whose visual progression did not match
the ten-band contract were discarded and were not copied into the project.

## Post-processing

The generator outputs were processed locally with ImageMagick:

1. identify the saturated magenta chroma field;
2. convert that field to alpha without erasing violet art details;
3. trim/pad the isolated sprite to a square composition;
4. reduce to exactly 80×80 using nearest-neighbour filtering;
5. retain sRGBA output;
6. inspect the ten-image contact sheet at
   `tmp/imagegen/lairs/contact-sheet.png`.

The ten final images are all 80×80 sRGBA PNGs with transparent backgrounds:

| Canonical art key / file | SHA-256 |
|---|---|
| `art_lair_visual_01_10.png` | `234b3a9222bfe519fc918971c9eead33bd71ae09c1a2223b7b3f99e6674faefb` |
| `art_lair_visual_11_20.png` | `e67afe32f0e0cb520d420e33a1634e76ddd65101c4d4e3960a3f5662da0d342e` |
| `art_lair_visual_21_30.png` | `fcd20b1daee3bec05bab4693fcb7cc0a1f4797de1fec6827af817fab0f55eae1` |
| `art_lair_visual_31_40.png` | `d73de430182ca130eddacb5f990baa237bfe0ddbd76be583682a20ae9187e102` |
| `art_lair_visual_41_50.png` | `024ea3e777763c233d63d4932f7d6485451734d9cc875084dd1615e791621769` |
| `art_lair_visual_51_60.png` | `ba1bb79d64855dc52c6dabc698f7c12c6a96174e6bcdcaee7d0d90a7f0351be3` |
| `art_lair_visual_61_70.png` | `0dab7711b01ebebe6fa82dc56bdbb77948f534df1f4ec9210f3d949db5acb76d` |
| `art_lair_visual_71_80.png` | `01b419c8381e0ea78f8ad943d5dc79b97fe4e43be8bc0b08ae9b997381d62961` |
| `art_lair_visual_81_90.png` | `aa42c47b0470cf1eb2b2bdc99b067ce99963770c6734afb6dbe4881eb97f850d` |
| `art_lair_visual_91_100.png` | `d7ad1302c766b48c0bfbcb240485e31adf55dc0cec045d7babf4ac2d601fcf21` |

## Runtime binding

`crates/cat-client/src/leader_ai_ui/lai68.rs` maps only the ten canonical
`art_lair_visual_*` report keys to these files. Unknown art keys keep the
existing textual/semantic fallback instead of inferring a band. The server
remains responsible for selecting the public band art key; the client never
uses hidden exact Lair state to choose a sprite.

This receipt does not close the rest of LAI.49. Creature portraits, material
and item icons, state sheets, badges, all-layout screenshots, and the remaining
asset matrix still require their own production assets and evidence.
