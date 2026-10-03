# Tools, packages and asset sources

No assets were purchased or generated through a paid AI service. Custom geometry
is authored by the project in Blender; see `source-art/README.md` and its manifest.
Third-party package source and binaries are resolved from package managers, not
vendored from local Library caches.

| Dependency | Source and license | Use |
| --- | --- | --- |
| Unity Editor 6000.6.0f1 | Unity Hub; Unity Editor terms under the user's existing account | Editor and native IL2CPP build |
| Unity CLI 1.0.0-beta.6 | Official Unity CLI distribution | Local Editor and batch commands |
| Pipeline 0.6.0-exp.1 | Unity registry; Unity Package Distribution License | Local Editor inspection and commands |
| Input System 1.20.0 | Unity registry; Unity Companion License | Keyboard, mouse and input state |
| Unity Test Framework 1.8.0 | Unity registry; Unity Companion License, bundled third-party notices | EditMode and PlayMode tests |
| Unity Newtonsoft JSON package 3.2.1 | Unity registry; Unity Companion License wrapper, Newtonsoft notices | Shared save and wire JSON |
| Newtonsoft.Json for .NET | NuGet; MIT | Server uses the same JSON contracts |
| .NET SDK 10.0.400 | Microsoft distribution through Homebrew; MIT and bundled third-party notices | C# server and standalone scenarios |
| Blender 5.2.1 LTS | Blender official distribution; GPL for the tool | Authored geometry and FBX exports |
| Rust 1.98.0 | Rust distribution through Homebrew; MIT/Apache-2.0 and dependency notices | One-time catalog export and legacy save normalization |

Package source licenses are available in each resolved package's `LICENSE.md`
and `Third Party Notices.md`. Engine packages stay within Unity-dependent project
use. The standalone server resolves Newtonsoft directly from NuGet. No runtime
AI provider, Behavior/GOAP package, paid art library or cloud account is required.

The lockfiles record the actual resolved graph, including transitive packages.
Catalog export reads this repository's maintained content rather than importing
an external content collection. Blender's tool license does not make the modeled
output Blender source code.

## Interface fonts and icons

The HUD ships two SIL Open Font Licence 1.1 families as TTF files under
`unity/Assets/Resources/UI/Fonts`, next to their licence texts. They are static
instances served by the Google Fonts CSS API (`fonts.googleapis.com/css2`), downloaded
and verified by `tools/ui-icons/fetch_fonts.sh`.

| File | Source URL | Author | Licence | SHA-256 |
| --- | --- | --- | --- | --- |
| `Manrope-Regular.ttf` (400) | https://fonts.gstatic.com/s/manrope/v20/xn7_YHE41ni1AdIRqAuZuw1Bx9mbZk79FO_F.ttf | The Manrope Project Authors (Mikhail Sharanda) | SIL OFL 1.1 | `a13d9b41b0a471ce58f0e46d377fa3cc76615e4632c3b15eb397caadf7a13f0a` |
| `Manrope-SemiBold.ttf` (600) | https://fonts.gstatic.com/s/manrope/v20/xn7_YHE41ni1AdIRqAuZuw1Bx9mbZk4jE-_F.ttf | The Manrope Project Authors (Mikhail Sharanda) | SIL OFL 1.1 | `6bad1a774228464cc88b8b0271555b266f7a3c64a7bedf15e165fdab4f6ac0ce` |
| `Manrope-Bold.ttf` (700) | https://fonts.gstatic.com/s/manrope/v20/xn7_YHE41ni1AdIRqAuZuw1Bx9mbZk4aE-_F.ttf | The Manrope Project Authors (Mikhail Sharanda) | SIL OFL 1.1 | `b9584e099e0e7f1e1e914c3037aabb9010c50346a3ce8df5eeeeecc0563044d8` |
| `Manrope-ExtraBold.ttf` (800) | https://fonts.gstatic.com/s/manrope/v20/xn7_YHE41ni1AdIRqAuZuw1Bx9mbZk59E-_F.ttf | The Manrope Project Authors (Mikhail Sharanda) | SIL OFL 1.1 | `b8f802f05b322a7345805f5b89f8bb976604cc719a91530918d5ce3c47160950` |
| `InstrumentSerif-Italic.ttf` | https://fonts.gstatic.com/s/instrumentserif/v5/jizHRFtNs2ka5fXjeivQ4LroWlx-6zATiw.ttf | The Instrument Serif Project Authors (Instrument) | SIL OFL 1.1 | `fecf4861d5202405b8bd45c683858b6870642a026784fe0dbf88ff55dedee8a0` |
| `OFL-Manrope.txt` | https://raw.githubusercontent.com/google/fonts/main/ofl/manrope/OFL.txt | The Manrope Project Authors | licence text | `58172e0c0fac2cda8a37b348164bb55e44b0e69051e557e92b1d3f6910141f7b` |
| `OFL-InstrumentSerif.txt` | https://raw.githubusercontent.com/google/fonts/main/ofl/instrumentserif/OFL.txt | The Instrument Serif Project Authors | licence text | `129ed7618959716959f2941fdd5b49e0ad6e6c1d78726761786a00253d865521` |

The interface icons are project-authored. `tools/ui-icons/build_icons.py` exports the
24 px line glyphs of `docs/ui-redesign/mockups/icons.js` to
`unity/Assets/Resources/UI/Icons/*.svg` (imported as UI Toolkit VectorImage) and copies the
tracked resource silhouettes `public/images/game/icons/*.png` to
`unity/Assets/Resources/UI/Icons/Goods/`; it also writes their importer metas with GUIDs
derived from the asset path. No external, purchased or generated art is involved.
