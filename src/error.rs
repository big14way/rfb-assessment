//! Application error types and the mapping from Bitcoin Core's JSON-RPC
//! error codes onto user-facing messages.

use thiserror::Error;

/// Bitcoin Core JSON-RPC error codes we treat specially.
///
/// These come from `src/rpc/protocol.h` in Bitcoin Core. Only the codes we can
/// give better-than-generic advice for are listed here; anything else falls
/// through to [`AppError::Rpc`].
pub mod codes {
    /// The method does not exist. Served over HTTP 404.
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// An argument had the right shape but the wrong JSON type.
    pub const TYPE_ERROR: i32 = -3;
    /// An argument was well-typed but out of range / otherwise unacceptable.
    pub const INVALID_PARAMETER: i32 = -8;
    /// The requested wallet is not loaded, or no wallet is loaded at all.
    pub const WALLET_NOT_FOUND: i32 = -18;
    /// More than one wallet is loaded and the call did not say which to use.
    pub const WALLET_NOT_SPECIFIED: i32 = -19;
}

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(
        "could not reach Bitcoin Core at {url}\n\
         hint: is your Polar network running? Check the RPC URL, or start the network in Polar."
    )]
    Connection {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error(
        "authentication failed for user '{user}'\n\
         hint: check the RPC username and password. Polar's defaults are polaruser / polarpass \
         (Polar shows them on the node's \"Connect\" tab)."
    )]
    Auth { user: String },

    #[error(
        "'{method}' is not a known Bitcoin Core RPC method\n\
         hint: run `rfb rpc help` to list every method this node supports."
    )]
    UnknownMethod { method: String },

    #[error(
        "wallet {wallet} is not loaded\n\
         hint: create or load a wallet in Polar, or select one with --wallet <name>. \
         Run `rfb rpc listwallets` to see what is currently loaded."
    )]
    WalletNotFound { wallet: String },

    #[error(
        "this node has more than one wallet loaded, so '{method}' needs to know which one to use\n\
         hint: pass --wallet <name>. Run `rfb rpc listwallets` to see what is loaded."
    )]
    WalletNotSpecified { method: String },

    #[error("invalid parameters for '{method}': {message}")]
    InvalidParams { method: String, message: String },

    #[error("Bitcoin Core rejected '{method}': {message} (code {code})")]
    Rpc {
        method: String,
        code: i32,
        message: String,
    },

    #[error(
        "could not understand the response to '{method}'\n\
         hint: this usually means the node runs a Bitcoin Core version whose response shape \
         differs from the one expected here."
    )]
    Decode {
        method: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("unexpected HTTP {status} response from Bitcoin Core: {body}")]
    Http { status: u16, body: String },

    #[error("invalid RPC URL '{url}': {message}")]
    InvalidUrl { url: String, message: String },

    #[error("could not read config file '{path}'")]
    ConfigRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("config file '{path}' is not valid TOML")]
    ConfigParse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
}

impl AppError {
    /// Translate a JSON-RPC error object into the most specific variant we have.
    ///
    /// `wallet` is the wallet the call was routed to, if any; it lets the
    /// "wallet not found" message name the wallet the user actually asked for.
    pub fn from_rpc(method: &str, wallet: Option<&str>, code: i32, message: String) -> Self {
        match code {
            codes::METHOD_NOT_FOUND => AppError::UnknownMethod {
                method: method.to_owned(),
            },
            codes::WALLET_NOT_FOUND => AppError::WalletNotFound {
                wallet: describe_wallet(wallet),
            },
            // Core's own message here explains how to pick a wallet via the
            // /wallet/<name> URI, which is exactly the detail --wallet exists
            // to hide, so it is replaced rather than passed through.
            codes::WALLET_NOT_SPECIFIED => AppError::WalletNotSpecified {
                method: method.to_owned(),
            },
            codes::TYPE_ERROR | codes::INVALID_PARAMETER => AppError::InvalidParams {
                method: method.to_owned(),
                message,
            },
            _ => AppError::Rpc {
                method: method.to_owned(),
                code,
                message,
            },
        }
    }
}

/// Render a wallet name for display. Bitcoin Core's default wallet has an empty
/// name, which would otherwise print as nothing at all.
fn describe_wallet(wallet: Option<&str>) -> String {
    match wallet {
        Some("") | None => "<default>".to_owned(),
        Some(name) => format!("'{name}'"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_method_code_maps_to_unknown_method() {
        let err = AppError::from_rpc(
            "nosuchmethod",
            None,
            codes::METHOD_NOT_FOUND,
            "Method not found".into(),
        );
        assert!(matches!(err, AppError::UnknownMethod { ref method } if method == "nosuchmethod"));
    }

    #[test]
    fn wallet_code_maps_to_wallet_not_found() {
        let err = AppError::from_rpc(
            "getbalance",
            Some("nope"),
            codes::WALLET_NOT_FOUND,
            "…".into(),
        );
        assert!(matches!(err, AppError::WalletNotFound { ref wallet } if wallet == "'nope'"));
    }

    /// The default wallet's name is the empty string, so it needs a placeholder
    /// rather than printing as nothing.
    #[test]
    fn default_wallet_is_named_in_errors() {
        let err = AppError::from_rpc("getbalance", Some(""), codes::WALLET_NOT_FOUND, "…".into());
        assert!(matches!(err, AppError::WalletNotFound { ref wallet } if wallet == "<default>"));
    }

    /// Passing a string where Core wants a number yields -3, not -8; both are
    /// the user's fault and both should read as an argument problem.
    #[test]
    fn type_and_range_codes_both_map_to_invalid_params() {
        for code in [codes::TYPE_ERROR, codes::INVALID_PARAMETER] {
            let err = AppError::from_rpc("getblockhash", None, code, "bad".into());
            assert!(
                matches!(err, AppError::InvalidParams { .. }),
                "code {code} should be InvalidParams"
            );
        }
    }

    /// Core answers -19 with advice about URI paths; this CLI should talk
    /// about --wallet instead.
    #[test]
    fn ambiguous_wallet_code_maps_to_wallet_not_specified() {
        let err = AppError::from_rpc("getbalance", None, codes::WALLET_NOT_SPECIFIED, "…".into());
        assert!(matches!(err, AppError::WalletNotSpecified { .. }));
        assert!(err.to_string().contains("--wallet"));
    }

    #[test]
    fn unrecognised_code_falls_through_to_generic_rpc_error() {
        let err = AppError::from_rpc("somemethod", None, -99, "weird".into());
        assert!(matches!(err, AppError::Rpc { code: -99, .. }));
    }
}
