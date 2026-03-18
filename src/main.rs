use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use alloy::rpc::types::eth::Filter;
use alloy::sol;
use alloy::sol_types::SolEvent;
use eyre::Result;
use futures::stream::StreamExt;
use std::env;
use dotenvy::dotenv;

sol! {
    event Approval(address indexed owner, address indexed spender, uint256 value);
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    println!("🚀 Starting tracker...");

    let alchemy_wss = env::var("ALCHEMY_WSS")
        .expect("❌ ERROR: ALCHEMY_WSS not found");

    // Establish a persistent WebSocket connection to the Ethereum Mainnet
    let ws = WsConnect::new(alchemy_wss);
    let provider = ProviderBuilder::new().on_ws(ws).await?;

    // Create a filter to tell the RPC node: "Only send us logs that match the ERC20 Approval signature"
    let filter = Filter::new().event_signature(Approval::SIGNATURE_HASH);
    
    // Subscribe to the live stream using our filter
    let mut stream = provider.subscribe_logs(&filter).await?.into_stream();

    println!("✅ Connected! Waiting for contract approvals...\n");

    // The Event Loop: This will run infinitely, processing new blocks and logs as they arrive
    while let Some(log) = stream.next().await {        
        // Check if the log is attached to a specific transaction hash
        if let Some(tx_hash) = log.transaction_hash {
            
            // Fetch the full transaction details from the RPC.
            if let Ok(Some(tx)) = provider.get_transaction_by_hash(tx_hash).await {
                
                // Extract the calldata (the raw hex input sent to the smart contract)
                let input_data = &tx.input;
                
                // The Selector Check: Verify the transaction's Method ID.
                // We ensure the calldata is at least 4 bytes long and matches `0x095ea7b3` (the `approve` function).
                if input_data.len() >= 4 && input_data[0..4] == [0x09, 0x5e, 0xa7, 0xb3] {
                    
                    // Decode the raw EVM log into readable Rust variables
                    if let Ok(decoded_log) = log.log_decode::<Approval>() {
                        let approval = decoded_log.inner;
                        
                        // Fetch the bytecode at the spender's address
                        if let Ok(code) = provider.get_code_at(approval.spender).await {
                            if code.is_empty() {
                                // Bytecode is empty -> This is an EOA.
                                // We skip this log and continue the loop.
                                continue; 
                            }
                        } else {
                            // If the RPC call fails for some reason, ignore and move on
                            continue;
                        }

                        let block_number = log.block_number.unwrap_or_default();

                        println!("🚨 CONTRACT APPROVAL DETECTED 🚨");
                        println!("Tx Hash         : {}", tx_hash);
                        println!("Block Number    : {}", block_number);
                        println!("Token Contract  : {}", log.address());
                        println!("Owner           : {}", approval.owner);
                        println!("Spender Contract: {}", approval.spender);
                        println!("Approved Amount : {}\n", approval.value);
                    }
                }
            }
        }
    }

    Ok(())
}