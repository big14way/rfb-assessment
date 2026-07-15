//! `wallet-info` and `balance`.

use super::{btc, heading, row, wallet_name};
use crate::error::Result;
use crate::rpc::RpcClient;
use serde::Deserialize;

/// The subset of `getwalletinfo` this command displays.
///
/// Note what is *not* here: `getwalletinfo` used to report `balance` and
/// `unconfirmed_balance`, but modern Bitcoin Core (checked against 30.0) no
/// longer returns them — `help getwalletinfo` lists neither. The balances come
/// from `getbalances` instead, which is why `wallet-info` makes two calls.
#[derive(Debug, Deserialize)]
pub struct WalletInfo {
    #[serde(rename = "walletname")]
    pub name: String,
    #[serde(rename = "txcount")]
    pub tx_count: u64,
}

/// `getbalances`, trimmed to the `mine` section.
///
/// The `watchonly` section only appears on wallets with watch-only keys, so it
/// is deliberately not modelled here.
#[derive(Debug, Deserialize)]
pub struct Balances {
    pub mine: MineBalances,
}

#[derive(Debug, Deserialize)]
pub struct MineBalances {
    /// Confirmed, spendable balance.
    pub trusted: f64,
    /// Incoming funds not yet confirmed.
    pub untrusted_pending: f64,
    /// Coinbase output not yet past its 100-block maturity.
    pub immature: f64,
}

/// Everything `wallet-info` displays, gathered from both calls.
#[derive(Debug)]
pub struct WalletSummary {
    pub name: String,
    pub tx_count: u64,
    pub balance: f64,
    pub unconfirmed_balance: f64,
    pub immature_balance: f64,
}

/// Fetch the wallet's identity and its balances, and fold them into one view.
async fn summary(client: &RpcClient, wallet: &str) -> Result<WalletSummary> {
    let info: WalletInfo = client.call(Some(wallet), "getwalletinfo", vec![]).await?;
    let balances: Balances = client.call(Some(wallet), "getbalances", vec![]).await?;

    Ok(WalletSummary {
        name: info.name,
        tx_count: info.tx_count,
        balance: balances.mine.trusted,
        unconfirmed_balance: balances.mine.untrusted_pending,
        immature_balance: balances.mine.immature,
    })
}

pub async fn info(client: &RpcClient, wallet: &str) -> Result<()> {
    let summary = summary(client, wallet).await?;

    heading("Wallet");
    row("Name", wallet_name(&summary.name));
    row("Balance", btc(summary.balance));
    row("Unconfirmed balance", btc(summary.unconfirmed_balance));
    row("Transactions", summary.tx_count);

    // Not asked for, but on regtest a freshly mined wallet holds most of its
    // funds here, and reporting only the spendable balance looks like a bug.
    if summary.immature_balance > 0.0 {
        row("Immature balance", btc(summary.immature_balance));
    }
    println!();

    Ok(())
}

pub async fn balance(client: &RpcClient, wallet: &str) -> Result<()> {
    let balance: f64 = client.call(Some(wallet), "getbalance", vec![]).await?;
    println!("{}", btc(balance));
    Ok(())
}
