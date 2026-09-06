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
