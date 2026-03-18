mod filters;

use alloy::primitives::Address;
use alloy::network::Ethereum;
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use alloy::transports::Transport;
use alloy::rpc::types::eth::Filter;
use alloy::sol;
use alloy::sol_types::SolEvent;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use dotenvy::dotenv;
use eyre::Result;
use futures::stream::StreamExt;
use serde::Serialize;
use std::env;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

use filters::{get_ignored_spenders, get_target_tokens};

// ~7200 blocks/day × 30 days
const BLOCKS_30_DAYS: u64 = 216_000;

/// Formats a raw token amount using its decimal places.
/// e.g. 1_000_000_000_000_000_000 with decimals=18 → "1"
fn format_amount(value: &alloy::primitives::U256, decimals: u8) -> String {
    if *value == alloy::primitives::U256::MAX {
        return "Unlimited".to_string();
    }
    let s = value.to_string();
    let d = decimals as usize;
    if d == 0 {
        return s;
    }
    let formatted = if s.len() <= d {
        let pad = "0".repeat(d - s.len());
        format!("0.{}{}", pad, s)
    } else {
        let (int, frac) = s.split_at(s.len() - d);
        format!("{}.{}", int, frac)
    };
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    trimmed.to_string()
}

/// Binary searches for the block at which `address` first had bytecode.
/// Returns `None` if the contract doesn't exist at `current_block`.
async fn find_deployment_block<T: Transport + Clone>(
    provider: &impl Provider<T, Ethereum>,
    address: Address,
    current_block: u64,
) -> Option<u64> {
    let mut low = 0u64;
    let mut high = current_block;

    while low < high {
        let mid = low + (high - low) / 2;
        let code = provider
            .get_code_at(address)
            .block_id(mid.into())
            .await
            .unwrap_or_default();

        if code.is_empty() {
            low = mid + 1;
        } else {
            high = mid;
        }
    }

    Some(low)
}

sol! {
    event Approval(address indexed owner, address indexed spender, uint256 value);
}

#[derive(Clone, Serialize)]
struct ApprovalEvent {
    tx_hash: String,
    block_number: u64,
    token_ticker: String,
    token_address: String,
    owner: String,
    spender: String,
    value: String,
    deployment_block: u64,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(tx): State<broadcast::Sender<String>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, tx))
}

async fn handle_socket(mut socket: WebSocket, tx: broadcast::Sender<String>) {
    let mut rx = tx.subscribe();
    while let Ok(msg) = rx.recv().await {
        if socket.send(Message::Text(msg.into())).await.is_err() {
            break;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    println!("🚀 Starting tracker...");

    let target_tokens = get_target_tokens();
    let ignored_spenders = get_ignored_spenders();
    let token_addresses: Vec<Address> = target_tokens.keys().copied().collect();

    let alchemy_wss = env::var("ALCHEMY_WSS").expect("❌ ERROR: ALCHEMY_WSS not found");

    // Broadcast channel: Rust backend → WebSocket clients
    let (event_tx, _) = broadcast::channel::<String>(100);
    let ws_tx = event_tx.clone();

    // Spawn the WebSocket server on port 3001
    tokio::spawn(async move {
        let app = Router::new()
            .route("/ws", get(ws_handler))
            .layer(CorsLayer::permissive())
            .with_state(ws_tx);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
        println!("🌐 WebSocket server listening on ws://localhost:3001/ws");
        axum::serve(listener, app).await.unwrap();
    });

    let ws = WsConnect::new(alchemy_wss);
    let provider = ProviderBuilder::new().on_ws(ws).await?;

    let filter = Filter::new()
        .event_signature(Approval::SIGNATURE_HASH)
        .address(token_addresses);

    let mut stream = provider.subscribe_logs(&filter).await?.into_stream();

    println!("✅ Connected! Waiting for contract approvals...\n");

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

        let Some(config) = target_tokens.get(&token_address) else {
            continue;
        };

        // 5. Apply business logic filters
        if approval.value < config.min_approval || ignored_spenders.contains(&approval.spender) {
            continue;
        }

        // 6. Verify the spender is a smart contract (has bytecode)
        let block_number = log.block_number.unwrap_or_default();
        match provider.get_code_at(approval.spender).await {
            Ok(code) if !code.is_empty() => {}
            _ => continue,
        }

        // 7. Only alert if the spender contract was deployed in the last 30 days
        let Some(deployment_block) =
            find_deployment_block(&provider, approval.spender, block_number).await
        else {
            continue;
        };

        if block_number.saturating_sub(deployment_block) > BLOCKS_30_DAYS {
            continue;
        }

        let formatted_value = format_amount(&approval.value, config.decimals);
        let blocks_old = block_number.saturating_sub(deployment_block);

        println!("🚨 CONTRACT APPROVAL DETECTED 🚨");
        println!("Tx Hash         : {}", tx_hash);
        println!("Block Number    : {}", block_number);
        println!("Token           : {} ({})", config.ticker, token_address);
        println!("Owner           : {}", approval.owner);
        println!("Spender Contract: {}", approval.spender);
        println!("Approved Amount : {} {}", formatted_value, config.ticker);
        println!("Contract Age    : {} blocks (~{} days)\n", blocks_old, blocks_old / 7200);

        let event = ApprovalEvent {
            tx_hash: tx_hash.to_string(),
            block_number,
            token_ticker: config.ticker.to_string(),
            token_address: token_address.to_string(),
            owner: approval.owner.to_string(),
            spender: approval.spender.to_string(),
            value: formatted_value,
            deployment_block,
        };
        let _ = event_tx.send(serde_json::to_string(&event).unwrap());
    }

    Ok(())
}
