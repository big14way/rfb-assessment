//! `new-address` — ask the wallet for a fresh receiving address.

use crate::error::Result;
use crate::rpc::RpcClient;
use serde_json::{Value, json};

pub async fn new_address(client: &RpcClient, wallet: &str, label: Option<String>) -> Result<()> {
    // getnewaddress takes (label, address_type), both optional. Only send the
    // label when there is one, so the node keeps its configured default type.
    let params: Vec<Value> = match label {
        Some(label) => vec![json!(label)],
        None => vec![],
    };

    let address: String = client.call(Some(wallet), "getnewaddress", params).await?;

    // Printed bare so the address can be piped straight into another command.
    println!("{address}");
    Ok(())
}
