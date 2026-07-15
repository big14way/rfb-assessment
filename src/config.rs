//! Connection settings, resolved from several sources.
//!
//! Precedence, highest first:
//!
//! 1. command-line flags
//! 2. environment variables
//! 3. a TOML config file
//! 4. built-in defaults
//!
//! Flags and environment variables are both handled by clap (see
//! [`crate::cli::ConnectionArgs`]), which already prefers a flag over the
//! matching variable. This module layers the file and the defaults underneath.

use crate::cli::ConnectionArgs;
use crate::error::{AppError, Result};
use serde::Deserialize;
use std::path::Path;

/// Defaults matching a stock Polar Bitcoin Core node, so that a freshly cloned
/// checkout works against Polar with no configuration at all.
pub const DEFAULT_RPC_URL: &str = "http://127.0.0.1:18443";
pub const DEFAULT_RPC_USER: &str = "polaruser";
pub const DEFAULT_RPC_PASSWORD: &str = "polarpass";

/// Config file consulted when `--config` is not given. A missing file here is
/// not an error; a missing *explicitly requested* file is.
pub const DEFAULT_CONFIG_PATH: &str = "config.toml";

/// The on-disk config file format.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    rpc_url: Option<String>,
    rpc_user: Option<String>,
    rpc_password: Option<String>,
    wallet: Option<String>,
}

/// Fully resolved connection settings.
#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub rpc_user: String,
    pub rpc_password: String,
    /// The wallet to talk to, if the user named one.
    ///
    /// `None` means "not specified", which the wallet commands read as the
    /// node's default wallet and the generic `rpc` command reads as "do not
    /// route through a wallet endpoint at all".
    pub wallet: Option<String>,
}

impl Config {
    /// Layer the config sources together.
    pub fn resolve(args: &ConnectionArgs) -> Result<Self> {
        let file = match &args.config {
            // Explicitly requested: it must exist.
            Some(path) => load_file(path, true)?,
            // Conventional location: use it if it happens to be there.
            None => load_file(Path::new(DEFAULT_CONFIG_PATH), false)?,
        };

        Ok(Config {
            rpc_url: pick(&args.rpc_url, file.rpc_url, DEFAULT_RPC_URL),
            rpc_user: pick(&args.rpc_user, file.rpc_user, DEFAULT_RPC_USER),
            rpc_password: pick(&args.rpc_password, file.rpc_password, DEFAULT_RPC_PASSWORD),
            wallet: args.wallet.clone().or(file.wallet),
        })
    }

    /// The wallet name to use for wallet-scoped commands.
    ///
    /// Bitcoin Core's default wallet is named with the empty string, and
    /// `/wallet/` is a valid endpoint that resolves to it, so the empty string
    /// is a correct fallback rather than a placeholder.
    pub fn wallet_or_default(&self) -> &str {
        self.wallet.as_deref().unwrap_or_default()
    }
}

/// First of: the flag/env value, the config file value, the built-in default.
fn pick(arg: &Option<String>, file: Option<String>, default: &str) -> String {
    arg.clone().or(file).unwrap_or_else(|| default.to_owned())
}

/// Read and parse a config file. When `required` is false, a missing file
/// yields an empty config rather than an error.
fn load_file(path: &Path, required: bool) -> Result<FileConfig> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !required => {
            return Ok(FileConfig::default());
        }
        Err(source) => {
            return Err(AppError::ConfigRead {
                path: display(path),
                source,
            });
        }
    };

    toml::from_str(&text).map_err(|source| AppError::ConfigParse {
        path: display(path),
        source,
    })
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_wins_over_file_and_default() {
        let arg = Some("http://flag".to_owned());
        assert_eq!(
            pick(&arg, Some("http://file".into()), "http://default"),
            "http://flag"
        );
    }

    #[test]
    fn file_wins_over_default() {
        assert_eq!(
            pick(&None, Some("http://file".into()), "http://default"),
            "http://file"
        );
    }

    #[test]
    fn default_is_the_last_resort() {
        assert_eq!(pick(&None, None, "http://default"), "http://default");
    }

    #[test]
    fn missing_optional_config_file_is_not_an_error() {
        let cfg = load_file(Path::new("definitely-not-here.toml"), false);
        assert!(cfg.is_ok());
    }

    #[test]
    fn missing_required_config_file_is_an_error() {
        let err = load_file(Path::new("definitely-not-here.toml"), true);
        assert!(matches!(err, Err(AppError::ConfigRead { .. })));
    }
}
