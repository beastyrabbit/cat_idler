//! Browser (WebAssembly) entry point for the Cat Colony Bevy client.
//!
//! Build with `trunk build --release` from `crates/cat-web/` (see `index.html`
//! and `docs/migration/WASM.md`), or a raw
//! `cargo build -p cat-web --target wasm32-unknown-unknown` for a compile check.
//! Bevy's `App::run()` does not block on wasm — winit drives it from
//! `requestAnimationFrame` — so calling it straight from `main` is correct.
fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    cat_client::run();
}
