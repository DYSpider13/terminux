mod app;
mod config;
mod ssh;
mod storage;
mod ui;
mod window;

use app::TerminuxApplication;
use gtk4::prelude::*;

fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Starting Terminux v{}", env!("CARGO_PKG_VERSION"));

    // Initialize GTK
    gtk4::init()?;

    // Create and run the application
    let app = TerminuxApplication::new();
    let exit_code = app.run();

    std::process::exit(exit_code.into());
}
