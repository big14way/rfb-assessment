//! `blockchain-info` — a summary of the node's view of the chain.

use super::{difficulty, heading, row};
use crate::error::Result;
use crate::rpc::RpcClient;
use serde::Deserialize;

/// The subset of `getblockchaininfo` this command displays.
///
/// Core returns a good deal more; serde ignores unknown fields by default,
/// which keeps this working across versions that add to the response.
#[derive(Debug, Deserialize)]
pub struct BlockchainInfo {
    pub chain: String,
    pub blocks: u64,
    pub headers: u64,
    pub difficulty: f64,
    #[serde(rename = "verificationprogress")]
    pub verification_progress: f64,
}

pub async fn info(client: &RpcClient) -> Result<()> {
    let info: BlockchainInfo = client.call(None, "getblockchaininfo", vec![]).await?;

    heading("Blockchain");
    row("Chain", &info.chain);
    row("Blocks", info.blocks);
    row("Headers", info.headers);
    row("Difficulty", difficulty(info.difficulty));
    row(
        "Verification progress",
        format!("{:.2}%", info.verification_progress * 100.0),
    );
    println!();

    Ok(())
}
