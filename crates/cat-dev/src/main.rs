//! Dev launcher: build + run `cat-server` and `cat-desktop` together with one command.
//!
//! ```text
//! cargo run -p cat-dev      # or: cargo dev   (alias in .cargo/config.toml)
//! ```
//!
//! It builds both binaries once, starts the server, waits until it is listening,
//! launches the desktop client (assets resolve from the workspace root), and stops
//! the server when the client window closes. `PORT` (default 8787), `GAME_DB_PATH`
//! (server default `data/cat.db`), and `WORKER_TICK_MS` are honoured from the
//! environment; no dependencies beyond std.

use std::env;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

fn main() {
    // Workspace root = crates/cat-dev/../.. — assets (`public/images/...`) and the
    // server's default `data/cat.db` are relative to it, so we run both children there.
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("cat-dev must live at crates/cat-dev")
        .to_path_buf();

    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8787);

    // 1. Build both binaries once (so the run below is instant and we can spawn the
    //    compiled binaries directly — that keeps cat-dev the real parent, so killing
    //    the server on exit actually works).
    eprintln!("[cat-dev] building cat-server + cat-desktop …");
    let built = Command::new("cargo")
        .args(["build", "-p", "cat-server", "-p", "cat-desktop"])
        .current_dir(&workspace_root)
        .status()
        .expect("failed to invoke cargo build");
    if !built.success() {
        eprintln!("[cat-dev] build failed");
        std::process::exit(1);
    }

    let target_dir = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("target"));
    let server_bin = target_dir.join("debug").join("cat-server");
    let client_bin = target_dir.join("debug").join("cat-desktop");

    // 2. Refuse to start if something is already listening on the port. That is a stale
    //    cat-server left over from an earlier run: our freshly-built server would fail to
    //    bind, but the readiness probe below would see the OLD process accepting
    //    connections and hand the client to it — serving pre-rebuild data (e.g. a snapshot
    //    missing fields we just added, which shows up as 0/0 staffing or absent columns).
    //    Fail loud instead of silently connecting to the wrong server.
    let addr = format!("127.0.0.1:{port}");
    if TcpStream::connect(&addr).is_ok() {
        eprintln!(
            "[cat-dev] a process is already listening on {addr} — that's a stale cat-server \
             from an earlier run. The client would connect to IT and serve outdated data. \
             Stop it first (e.g. `pkill -f target/debug/cat-server`, or free PORT), then re-run."
        );
        std::process::exit(1);
    }

    // 3. Start the server (inherits GAME_DB_PATH / WORKER_TICK_MS from the environment).
    eprintln!("[cat-dev] starting cat-server on 127.0.0.1:{port} …");
    let mut server = Command::new(&server_bin)
        .current_dir(&workspace_root)
        .env("PORT", port.to_string())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("failed to start cat-server");

    // 4. Wait until it accepts connections (or crashes / times out).
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if TcpStream::connect(&addr).is_ok() {
            break;
        }
        if let Ok(Some(status)) = server.try_wait() {
            eprintln!("[cat-dev] cat-server exited before it was ready ({status})");
            std::process::exit(1);
        }
        if Instant::now() >= deadline {
            eprintln!("[cat-dev] cat-server did not start listening on {addr} within 30s");
            let _ = server.kill();
            std::process::exit(1);
        }
        sleep(Duration::from_millis(150));
    }
    eprintln!("[cat-dev] server ready — launching cat-desktop …");

    // 5. Run the client in the foreground; it talks to the server over the WS.
    //    Bevy resolves the AssetPlugin `file_path` (".") against BEVY_ASSET_ROOT, else
    //    the inherited CARGO_MANIFEST_DIR (which cargo set to crates/cat-dev) — NOT the
    //    CWD. So point the asset root at the workspace so public/images/* resolves.
    let client_status = Command::new(&client_bin)
        .current_dir(&workspace_root)
        .env("BEVY_ASSET_ROOT", &workspace_root)
        .env_remove("CARGO_MANIFEST_DIR")
        .env("CAT_SERVER_URL", format!("ws://127.0.0.1:{port}/ws"))
        .status();

    // 6. Client closed → stop the server.
    eprintln!("[cat-dev] client exited — stopping cat-server …");
    let _ = server.kill();
    let _ = server.wait();

    match client_status {
        Ok(s) if !s.success() => std::process::exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("[cat-dev] failed to run cat-desktop: {e}");
            std::process::exit(1);
        }
        _ => {}
    }
}
