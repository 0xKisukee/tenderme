mod filters;

use alloy::primitives::{address, Address};
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use alloy::rpc::types::eth::Filter;
use alloy::sol;
use alloy::sol_types::SolEvent;
use dotenvy::dotenv;
use eyre::Result;
use futures::stream::StreamExt;
use std::env;

use filters::{get_ignored_spenders, get_target_tokens};

sol! {
    event Approval(address indexed owner, address indexed spender, uint256 value);
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    println!("🚀 Starting tracker...");

    // Load configurations from our filters module
    let target_tokens = get_target_tokens();
    let ignored_spenders = get_ignored_spenders();
    let token_addresses: Vec<Address> = target_tokens.keys().copied().collect();

    let alchemy_wss = env::var("ALCHEMY_WSS").expect("❌ ERROR: ALCHEMY_WSS not found");

    // Establish a persistent WebSocket connection to the Ethereum Mainnet
    let ws = WsConnect::new(alchemy_wss);
    let provider = ProviderBuilder::new().on_ws(ws).await?;

    // Create a filter for ERC20 Approval signatures on our target tokens
    let filter = Filter::new()
        .event_signature(Approval::SIGNATURE_HASH)
        .address(token_addresses);

    let mut stream = provider.subscribe_logs(&filter).await?.into_stream();

    println!("✅ Connected! Waiting for contract approvals...\n");

    // The Event Loop
    while let Some(log) = stream.next().await {
        // 1. Ensure log has a transaction hash
        let Some(tx_hash) = log.transaction_hash else {
            continue;
        };

        // 2. Fetch the full transaction details
        let Ok(Some(tx)) = provider.get_transaction_by_hash(tx_hash).await else {
            continue;
        };

        // 3. Verify it's an `approve` function call (0x095ea7b3)
        let input_data = &tx.input;
        if input_data.len() < 4 || input_data[0..4] != [0x09, 0x5e, 0xa7, 0xb3] {
            continue;
        }

        // 4. Decode the log
        let Ok(decoded_log) = log.log_decode::<Approval>() else {
            continue;
        };
        let approval = decoded_log.inner;
        let token_address = log.address();
        
        // Safety check: skip if we somehow get a log for an unconfigured token
        let Some(config) = target_tokens.get(&token_address) else {
            continue;
        };

        // 5. Apply business logic filters
        if approval.value < config.min_approval || ignored_spenders.contains(&approval.spender) {
            continue;
        }

        // 6. Verify the spender is a smart contract (has bytecode)
        match provider.get_code_at(approval.spender).await {
            Ok(code) if !code.is_empty() => {} // Spender is a contract, proceed
            _ => continue, // Spender is an EOA or call failed, skip
        }

        // --- Alert Execution ---
        let block_number = log.block_number.unwrap_or_default();

        println!("🚨 CONTRACT APPROVAL DETECTED 🚨");
        println!("Tx Hash         : {}", tx_hash);
        println!("Block Number    : {}", block_number);
        println!("Token           : {} ({})", config.ticker, token_address);
        println!("Owner           : {}", approval.owner);
        println!("Spender Contract: {}", approval.spender);
        println!("Approved Amount : {}\n", approval.value);
    }

    Ok(())
}