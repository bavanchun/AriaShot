use camerashot_cli::Cli;
use camerashot_platform::create_default_backend;
use clap::Parser;
use std::io::{stderr, stdout};

fn main() {
    // Initialise tracing with default warn filter.
    // Set RUST_LOG=debug (etc.) to increase verbosity.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .with_writer(stderr)
        .init();

    let cli = Cli::parse();

    let backend = match create_default_backend() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: failed to create capture backend: {e}");
            std::process::exit(1);
        }
    };

    let code = camerashot_cli::run(&cli, &*backend, &mut stdout(), &mut stderr());
    std::process::exit(code);
}
