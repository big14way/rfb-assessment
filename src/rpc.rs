//! A small, reusable Bitcoin Core JSON-RPC client.
//!
//! The whole client is one generic method, [`RpcClient::call`], which
//! deserializes into whatever the caller asks for. Typed commands ask for a
//! struct; the generic `rpc` passthrough asks for [`serde_json::Value`]. That
//! keeps a single code path responsible for transport, auth and error mapping.

use crate::config::Config;
use crate::error::{AppError, Result};
use reqwest::{StatusCode, Url};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::time::Duration;
use tracing::debug;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// JSON-RPC 1.0, which is what Bitcoin Core speaks.
#[derive(Debug, Serialize)]
struct Request<'a> {
    jsonrpc: &'a str,
    id: &'a str,
    method: &'a str,
    params: &'a [Value],
}

/// A response envelope. Parsed with `result` left as a raw [`Value`] so that a
/// node-side error and a client-side deserialization mismatch stay
/// distinguishable — see [`RpcClient::call`].
#[derive(Debug, serde::Deserialize)]
struct Response {
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<ResponseError>,
}

#[derive(Debug, serde::Deserialize)]
struct ResponseError {
    code: i32,
    message: String,
}

#[derive(Debug, Clone)]
pub struct RpcClient {
    http: reqwest::Client,
    base_url: Url,
    user: String,
    password: String,
}

impl RpcClient {
    pub fn new(config: &Config) -> Result<Self> {
        let base_url = Url::parse(&config.rpc_url).map_err(|e| AppError::InvalidUrl {
            url: config.rpc_url.clone(),
            message: e.to_string(),
        })?;

        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|source| AppError::Connection {
                url: config.rpc_url.clone(),
                source,
            })?;

        Ok(Self {
            http,
            base_url,
            user: config.rpc_user.clone(),
            password: config.rpc_password.clone(),
        })
    }

    /// Call `method`, deserializing the result into `T`.
    ///
    /// `wallet` selects the endpoint: `Some(name)` routes to `/wallet/<name>`
    /// (required for wallet RPCs once more than one wallet is loaded), `None`
    /// posts to the root endpoint.
    pub async fn call<T: DeserializeOwned>(
        &self,
        wallet: Option<&str>,
        method: &str,
        params: Vec<Value>,
    ) -> Result<T> {
        let url = self.endpoint(wallet)?;
        // Bound to a local because `tracing`'s `%` sigil takes a value, not a
        // constructor call.
        let logged_params = Value::Array(params.clone());
        debug!(%url, method, params = %logged_params, "rpc request");

        let response = self
            .http
            .post(url)
            .basic_auth(&self.user, Some(&self.password))
            .json(&Request {
                jsonrpc: "1.0",
                id: "rfb",
                method,
                params: &params,
            })
            .send()
            .await
            .map_err(|source| AppError::Connection {
                url: self.base_url.to_string(),
                source,
            })?;

        let status = response.status();

        // Bad credentials are the one failure Core answers with an empty body,
        // so this has to come before any attempt to parse JSON.
        if status == StatusCode::UNAUTHORIZED {
            return Err(AppError::Auth {
                user: self.user.clone(),
            });
        }

        let body = response
            .bytes()
            .await
            .map_err(|source| AppError::Connection {
                url: self.base_url.to_string(),
                source,
            })?;
        debug!(%status, body = %String::from_utf8_lossy(&body), "rpc response");

        // Core reports RPC errors with a non-2xx status *and* a JSON body
        // describing what went wrong (404 for an unknown method, 500 for most
        // of the rest). Bailing out on status alone would throw that away, so
        // the body is parsed regardless and the status is only consulted if it
        // turns out not to be JSON-RPC at all.
        let Ok(parsed) = serde_json::from_slice::<Response>(&body) else {
            return Err(AppError::Http {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&body).trim().to_owned(),
            });
        };

        if let Some(error) = parsed.error {
            return Err(AppError::from_rpc(
                method,
                wallet,
                error.code,
                error.message,
            ));
        }

        serde_json::from_value(parsed.result.unwrap_or(Value::Null)).map_err(|source| {
            AppError::Decode {
                method: method.to_owned(),
                source,
            }
        })
    }

    /// Build the endpoint URL, appending `/wallet/<name>` when a wallet is
    /// selected. `path_segments_mut` percent-encodes the name, so wallets with
    /// awkward characters route correctly; pushing an empty segment yields
    /// `/wallet/`, which is how Core addresses the default wallet.
    fn endpoint(&self, wallet: Option<&str>) -> Result<Url> {
        let mut url = self.base_url.clone();
        if let Some(name) = wallet {
            url.path_segments_mut()
                .map_err(|_| AppError::InvalidUrl {
                    url: self.base_url.to_string(),
                    message: "URL cannot have a path appended to it".to_owned(),
                })?
                .pop_if_empty()
                .push("wallet")
                .push(name);
        }
        Ok(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> RpcClient {
        RpcClient::new(&Config {
            rpc_url: "http://127.0.0.1:18443".into(),
            rpc_user: "polaruser".into(),
            rpc_password: "polarpass".into(),
            wallet: None,
        })
        .expect("test config is valid")
    }

    #[test]
    fn no_wallet_posts_to_the_root_endpoint() {
        assert_eq!(
            client().endpoint(None).unwrap().as_str(),
            "http://127.0.0.1:18443/"
        );
    }

    /// The default wallet has an empty name and lives at `/wallet/`.
    #[test]
    fn default_wallet_routes_to_the_bare_wallet_endpoint() {
        assert_eq!(
            client().endpoint(Some("")).unwrap().as_str(),
            "http://127.0.0.1:18443/wallet/"
        );
    }

    #[test]
    fn named_wallet_is_appended_to_the_path() {
        assert_eq!(
            client().endpoint(Some("alice")).unwrap().as_str(),
            "http://127.0.0.1:18443/wallet/alice"
        );
    }

    #[test]
    fn wallet_names_are_percent_encoded() {
        assert_eq!(
            client().endpoint(Some("my wallet")).unwrap().as_str(),
            "http://127.0.0.1:18443/wallet/my%20wallet"
        );
    }

    #[test]
    fn invalid_url_is_rejected_up_front() {
        let err = RpcClient::new(&Config {
            rpc_url: "not a url".into(),
            rpc_user: String::new(),
            rpc_password: String::new(),
            wallet: None,
        });
        assert!(matches!(err, Err(AppError::InvalidUrl { .. })));
    }
}

/// Transport tests, run against a stub HTTP server.
///
/// Bitcoin Core reports most failures with a non-2xx status *and* a meaningful
/// JSON body, and reports exactly one (bad credentials) with an empty body.
/// Those combinations are awkward to provoke from a live node on demand, so
/// they are pinned here instead.
#[cfg(test)]
mod transport_tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;
    use wiremock::matchers::{body_json, header, method as http_method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn client_for(server: &MockServer) -> RpcClient {
        RpcClient::new(&Config {
            rpc_url: server.uri(),
            rpc_user: "polaruser".into(),
            rpc_password: "polarpass".into(),
            wallet: None,
        })
        .expect("mock server URI is valid")
    }

    /// Answer any POST with this status and JSON body.
    async fn respond_with(server: &MockServer, status: u16, body: serde_json::Value) {
        Mock::given(http_method("POST"))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn successful_call_deserializes_into_the_requested_type() {
        let server = MockServer::start().await;
        respond_with(
            &server,
            200,
            json!({"result": 101, "error": null, "id": "rfb"}),
        )
        .await;

        let count: u64 = client_for(&server)
            .call(None, "getblockcount", vec![])
            .await
            .expect("call should succeed");

        assert_eq!(count, 101);
    }

    /// The same call path has to serve typed structs too — that is the whole
    /// point of `call` being generic.
    #[tokio::test]
    async fn the_same_call_path_deserializes_a_struct() {
        #[derive(Debug, Deserialize)]
        struct Info {
            chain: String,
            blocks: u64,
        }

        let server = MockServer::start().await;
        respond_with(
            &server,
            200,
            json!({"result": {"chain": "regtest", "blocks": 101}, "error": null, "id": "rfb"}),
        )
        .await;

        let info: Info = client_for(&server)
            .call(None, "getblockchaininfo", vec![])
            .await
            .expect("call should succeed");

        assert_eq!(info.chain, "regtest");
        assert_eq!(info.blocks, 101);
    }

    /// Core speaks JSON-RPC 1.0, not 2.0.
    #[tokio::test]
    async fn request_is_sent_as_json_rpc_1_0_with_the_given_params() {
        let server = MockServer::start().await;
        Mock::given(http_method("POST"))
            .and(body_json(json!({
                "jsonrpc": "1.0",
                "id": "rfb",
                "method": "getblockhash",
                "params": [200],
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"result": "abc", "error": null, "id": "rfb"})),
            )
            .mount(&server)
            .await;

        let hash: String = client_for(&server)
            .call(None, "getblockhash", vec![json!(200)])
            .await
            .expect("body should match the expected envelope");

        assert_eq!(hash, "abc");
    }

    #[tokio::test]
    async fn credentials_are_sent_as_http_basic_auth() {
        let server = MockServer::start().await;
        Mock::given(http_method("POST"))
            // base64("polaruser:polarpass")
            .and(header(
                "authorization",
                "Basic cG9sYXJ1c2VyOnBvbGFycGFzcw==",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"result": 1, "error": null})),
            )
            .mount(&server)
            .await;

        let result: u64 = client_for(&server)
            .call(None, "getblockcount", vec![])
            .await
            .expect("basic auth header should match");

        assert_eq!(result, 1);
    }

    /// 401 is the one failure Core answers with an empty body, so it must be
    /// caught before anything tries to parse JSON.
    #[tokio::test]
    async fn unauthorized_with_an_empty_body_maps_to_an_auth_error() {
        let server = MockServer::start().await;
        Mock::given(http_method("POST"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let err = client_for(&server)
            .call::<Value>(None, "getblockcount", vec![])
            .await
            .expect_err("401 should fail");

        assert!(matches!(err, AppError::Auth { .. }), "got {err:?}");
    }

    /// The regression test for the whole design: Core serves "method not found"
    /// over HTTP 404 with the real error in the body. Bailing out on status
    /// would discard it.
    #[tokio::test]
    async fn method_not_found_over_http_404_still_reads_the_body() {
        let server = MockServer::start().await;
        respond_with(
            &server,
            404,
            json!({"result": null, "error": {"code": -32601, "message": "Method not found"}}),
        )
        .await;

        let err = client_for(&server)
            .call::<Value>(None, "notarealmethod", vec![])
            .await
            .expect_err("unknown method should fail");

        assert!(
            matches!(err, AppError::UnknownMethod { ref method } if method == "notarealmethod"),
            "got {err:?}"
        );
    }

    /// Likewise for HTTP 500, which carries most of Core's errors.
    #[tokio::test]
    async fn wallet_error_over_http_500_still_reads_the_body() {
        let server = MockServer::start().await;
        respond_with(
            &server,
            500,
            json!({"result": null, "error": {"code": -18, "message": "Requested wallet does not exist"}}),
        )
        .await;

        let err = client_for(&server)
            .call::<Value>(Some("ghost"), "getbalance", vec![])
            .await
            .expect_err("missing wallet should fail");

        assert!(
            matches!(err, AppError::WalletNotFound { ref wallet } if wallet == "'ghost'"),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn invalid_params_over_http_500_maps_to_invalid_params() {
        let server = MockServer::start().await;
        respond_with(
            &server,
            500,
            json!({"result": null, "error": {"code": -8, "message": "Block height out of range"}}),
        )
        .await;

        let err = client_for(&server)
            .call::<Value>(None, "getblockhash", vec![json!(999)])
            .await
            .expect_err("bad params should fail");

        assert!(matches!(err, AppError::InvalidParams { .. }), "got {err:?}");
    }

    /// A proxy or a wrong port can return something that is not JSON-RPC at
    /// all; that should surface as an HTTP error rather than a parse failure.
    #[tokio::test]
    async fn a_non_json_body_maps_to_an_http_error() {
        let server = MockServer::start().await;
        Mock::given(http_method("POST"))
            .respond_with(ResponseTemplate::new(502).set_body_string("<html>bad gateway</html>"))
            .mount(&server)
            .await;

        let err = client_for(&server)
            .call::<Value>(None, "getblockcount", vec![])
            .await
            .expect_err("non-JSON body should fail");

        assert!(
            matches!(err, AppError::Http { status: 502, ref body } if body.contains("bad gateway")),
            "got {err:?}"
        );
    }

    /// A node-side error and a client-side shape mismatch are different
    /// problems and must not be conflated.
    #[tokio::test]
    async fn a_result_of_the_wrong_shape_maps_to_a_decode_error() {
        let server = MockServer::start().await;
        respond_with(
            &server,
            200,
            json!({"result": "not-a-number", "error": null}),
        )
        .await;

        let err = client_for(&server)
            .call::<u64>(None, "getblockcount", vec![])
            .await
            .expect_err("wrong result type should fail");

        assert!(
            matches!(err, AppError::Decode { ref method, .. } if method == "getblockcount"),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn wallet_calls_are_routed_to_the_wallet_path() {
        let server = MockServer::start().await;
        Mock::given(http_method("POST"))
            .and(path("/wallet/alice"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"result": 0.5, "error": null})),
            )
            .mount(&server)
            .await;

        let balance: f64 = client_for(&server)
            .call(Some("alice"), "getbalance", vec![])
            .await
            .expect("request should reach /wallet/alice");

        assert_eq!(balance, 0.5);
    }

    #[tokio::test]
    async fn non_wallet_calls_are_routed_to_the_root_path() {
        let server = MockServer::start().await;
        Mock::given(http_method("POST"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"result": 101, "error": null})),
            )
            .mount(&server)
            .await;

        let count: u64 = client_for(&server)
            .call(None, "getblockcount", vec![])
            .await
            .expect("request should reach /");

        assert_eq!(count, 101);
    }

    /// Nothing is listening on port 1, so this exercises the connect path.
    #[tokio::test]
    async fn a_refused_connection_maps_to_a_connection_error() {
        let client = RpcClient::new(&Config {
            rpc_url: "http://127.0.0.1:1".into(),
            rpc_user: "polaruser".into(),
            rpc_password: "polarpass".into(),
            wallet: None,
        })
        .expect("URL is valid");

        let err = client
            .call::<Value>(None, "getblockcount", vec![])
            .await
            .expect_err("connecting to a closed port should fail");

        assert!(matches!(err, AppError::Connection { .. }), "got {err:?}");
    }
}
