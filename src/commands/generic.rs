//! `rpc` — call any method the node exposes.

use crate::error::Result;
use crate::rpc::RpcClient;
use serde_json::Value;

/// Turn one command-line argument into a JSON value.
///
/// Bitcoin Core is strict about argument types: `getblockhash "200"` is
/// rejected with "JSON value of type string is not of expected type number",
/// while `getblock <hash>` genuinely wants a string. Command-line arguments
/// arrive as text either way, so each one is offered to the JSON parser first
/// and kept as a string only if it is not valid JSON.
///
/// The rule is deliberately simple, and it has one known corner: an argument
/// that is *all* digits is always sent as a number. Every identifier Core takes
/// as a string (block hashes, txids, addresses) is hex or base58/bech32 and in
/// practice contains at least one non-digit, so this has not needed a
/// workaround — but `--` style quoting would be the escape hatch if it did.
pub fn parse_param(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_owned()))
}

pub async fn run(
    client: &RpcClient,
    wallet: Option<&str>,
    method: &str,
    params: &[String],
) -> Result<()> {
    let params: Vec<Value> = params.iter().map(|raw| parse_param(raw)).collect();
    let result: Value = client.call(wallet, method, params).await?;

    match result {
        // Unwrapped, so `rpc getblockhash 200 | xargs rfb rpc getblock` works
        // rather than piping a quoted string on.
        Value::String(s) => println!("{s}"),
        other => println!(
            "{}",
            serde_json::to_string_pretty(&other).unwrap_or_else(|_| other.to_string())
        ),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The case that motivates the whole function: Core rejects "200" but
    /// accepts 200.
    #[test]
    fn integers_become_json_numbers() {
        assert_eq!(parse_param("200"), json!(200));
    }

    #[test]
    fn block_hashes_stay_strings() {
        let hash = "7ad1d1efddaa8612f46c0e66473a371aaf1c1ef61b8317609a5d3081d912bca1";
        assert_eq!(parse_param(hash), json!(hash));
    }

    #[test]
    fn booleans_are_recognised() {
        assert_eq!(parse_param("true"), json!(true));
        assert_eq!(parse_param("false"), json!(false));
    }

    #[test]
    fn floats_are_recognised() {
        assert_eq!(parse_param("0.5"), json!(0.5));
    }

    #[test]
    fn negative_numbers_are_recognised() {
        assert_eq!(parse_param("-1"), json!(-1));
    }

    #[test]
    fn arrays_and_objects_pass_through_as_json() {
        assert_eq!(parse_param("[1,2]"), json!([1, 2]));
        assert_eq!(parse_param(r#"{"a":1}"#), json!({"a": 1}));
    }

    /// Bare words are not valid JSON, so they survive as strings.
    #[test]
    fn plain_words_stay_strings() {
        assert_eq!(parse_param("getblockcount"), json!("getblockcount"));
        assert_eq!(
            parse_param("bcrt1qakk8gz9eal0n457ezkqnenasyen32sfeplec03"),
            json!("bcrt1qakk8gz9eal0n457ezkqnenasyen32sfeplec03")
        );
    }

    /// An explicitly quoted argument is JSON for a string, and should not end
    /// up double-quoted.
    #[test]
    fn quoted_arguments_are_unwrapped_once() {
        assert_eq!(parse_param(r#""200""#), json!("200"));
    }
}
