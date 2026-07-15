//! A command-line client for a Bitcoin Core node's JSON-RPC interface,
//! aimed at a regtest node run by Polar.

mod cli;
mod commands;
mod config;
mod error;
mod rpc;

use clap::Parser;
use cli::{Cli, Command};
use config::Config;
use error::Result;
use rpc::RpcClient;
use std::process::ExitCode;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            report(&error);
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<()> {
    let config = Config::resolve(&cli.connection)?;
    let client = RpcClient::new(&config)?;

    match cli.command {
        Command::BlockchainInfo => commands::blockchain::info(&client).await,
        Command::WalletInfo => commands::wallet::info(&client, config.wallet_or_default()).await,
        Command::Balance => commands::wallet::balance(&client, config.wallet_or_default()).await,
        Command::NewAddress { label } => {
            commands::address::new_address(&client, config.wallet_or_default(), label).await
        }
        // The passthrough only routes through a wallet endpoint when the user
        // named a wallet; otherwise it posts to the root, so non-wallet methods
        // behave exactly as they would through bitcoin-cli.
        Command::Rpc { method, params } => {
            commands::generic::run(&client, config.wallet.as_deref(), &method, &params).await
        }
    }
}

/// Print an error and its causes to stderr.
///
/// Errors are reported by hand rather than by returning `Result` from `main`,
/// which would `Debug`-format them and print the enum variant alongside the
/// message.
fn report(error: &dyn std::error::Error) {
    eprintln!("error: {error}");

    let mut source = error.source();
    while let Some(cause) = source {
        eprintln!("  caused by: {cause}");
        source = cause.source();
    }
}

/// Warnings only by default; `-v` turns on debug logging for the RPC layer.
/// `RUST_LOG` overrides both.
fn init_tracing(verbose: bool) {
    let default = if verbose { "rfb=debug" } else { "warn" };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .with_writer(std::io::stderr)
        .init();
}
