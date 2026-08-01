//! Deterministic LAI.33 browser fixture writer.

use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let database = PathBuf::from(args.next().ok_or("database path is required")?);
    let manifest = PathBuf::from(args.next().ok_or("manifest path is required")?);
    cat_server::leader_ai_journey::write_lai33_authoritative_fixture(&database, &manifest)
}
