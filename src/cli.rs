//! Command-line surface, defined with clap's derive API.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "rfb",
    version,
    about = "Talk to a Bitcoin Core node over JSON-RPC",
    long_about = "A small command-line client for a Bitcoin Core node, aimed at a regtest \
                  node run by Polar.\n\n\
                  Connection settings are resolved from command-line flags, then environment \
                  variables, then a config file, then built-in defaults that match Polar."
)]
pub struct Cli {
    #[command(flatten)]
    pub connection: ConnectionArgs,

    /// Print RPC requests and responses to stderr
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}

/// Connection settings, shared by every subcommand.
///
/// Every field is an `Option` so that "not given here" stays distinguishable
/// from "given, and happens to equal the default" — [`crate::config`] needs that
/// distinction to layer the sources in the right order.
#[derive(Debug, Args)]
pub struct ConnectionArgs {
    /// Bitcoin Core JSON-RPC URL
    #[arg(long, env = "BITCOIN_RPC_URL", global = true, value_name = "URL")]
    pub rpc_url: Option<String>,

    /// RPC username
    #[arg(long, env = "BITCOIN_RPC_USER", global = true, value_name = "USER")]
    pub rpc_user: Option<String>,

    /// RPC password
    #[arg(
        long,
        env = "BITCOIN_RPC_PASSWORD",
        global = true,
        value_name = "PASSWORD",
        hide_env_values = true
    )]
    pub rpc_password: Option<String>,

    /// Wallet to use for wallet commands (defaults to the node's default wallet)
    #[arg(long, env = "BITCOIN_RPC_WALLET", global = true, value_name = "NAME")]
    pub wallet: Option<String>,

    /// Path to a TOML config file (defaults to ./config.toml when present)
    #[arg(long, env = "RFB_CONFIG", global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Show chain, block height, headers, difficulty and verification progress
    BlockchainInfo,

    /// Show the wallet's name, balances and transaction count
    WalletInfo,

    /// Print the wallet balance
    Balance,

    /// Generate and print a new receiving address
    NewAddress {
        /// Optional label to file the address under in the wallet
        #[arg(long, value_name = "LABEL")]
        label: Option<String>,
    },

    /// Call an arbitrary RPC method
    ///
    /// Arguments are parsed as JSON where possible and sent as strings
    /// otherwise, so `rpc getblockhash 200` sends a number while
    /// `rpc getblock <hash>` sends a string.
    #[command(long_about = "Call an arbitrary RPC method.\n\n\
                      Arguments are interpreted as JSON when they parse as JSON, and sent as \
                      strings when they do not. That means `rpc getblockhash 200` sends the \
                      number 200 (which Core requires), while `rpc getblock <hash>` sends a \
                      string.\n\n\
                      Because every argument after the method name is forwarded to the node, \
                      connection flags must come before the subcommand:\n\
                      \x20 rfb --rpc-url http://127.0.0.1:18443 rpc getblockcount")]
    Rpc {
        /// RPC method name, e.g. getblockcount
        method: String,

        /// Arguments to forward to the method
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            value_name = "ARGS"
        )]
        params: Vec<String>,
    },
}
