use crate::simulation::models::{MarketType, TransactionResult};
use ethers::types::{Address, U256};
use rand::Rng;
use std::str::FromStr;

/// Convert ETH amount to Wei as U256
pub fn eth_to_wei(eth: f64) -> U256 {
    let wei = (eth * 1e18) as u128;
    U256::from(wei)
}

/// Convert Wei to ETH
pub fn wei_to_eth(wei: U256) -> f64 {
    let wei_value = wei.as_u128() as f64;
    wei_value / 1e18
}

/// Generate a random ETH amount between min and max
pub fn random_eth_amount(min_eth: f64, max_eth: f64) -> U256 {
    let mut rng = rand::thread_rng();
    let eth_amount = rng.gen_range(min_eth..=max_eth);
    eth_to_wei(eth_amount)
}

/// Format U256 as ETH string with precision
pub fn format_eth(wei: U256) -> String {
    let eth = wei_to_eth(wei);
    format!("{:.8}", eth)
}

/// Format U256 as token string with precision
pub fn format_tokens(tokens: U256) -> String {
    let token_value = tokens.as_u128() as f64;
    let display_value = token_value / 1e18; // Assuming 18 decimals
    format!("{:.8}", display_value)
}

/// Calculate fee based on basis points
pub fn calculate_fee(amount: U256, basis_points: u16) -> U256 {
    amount * U256::from(basis_points) / U256::from(10_000)
}

/// Determine if a referral should be included based on probability
pub fn should_include_referral(probability: i32) -> bool {
    let mut rng = rand::thread_rng();
    rng.gen_ratio(probability as u32, 100)
}

/// Parse an Ethereum address from string
pub fn parse_address(address: &str) -> Option<Address> {
    Address::from_str(address).ok()
}

/// Log a transaction result to console
pub fn log_transaction_result(result: &TransactionResult) {
    let status = if result.success { "SUCCESS" } else { "FAILED" };
    let tx_hash = result.tx_hash
        .map(|h| format!("{:?}", h))
        .unwrap_or_else(|| "None".to_string());
    
    let tokens = result.tokens_received
        .map(|t| format_tokens(t))
        .unwrap_or_else(|| "0".to_string());
    
    let error = result.error
        .clone()
        .unwrap_or_else(|| "".to_string());
    
    let market_type = match result.market_type {
        MarketType::BondingCurve => "BONDING_CURVE",
        MarketType::UniswapPool => "UNISWAP_POOL",
    };
    
    log::info!("Transaction: {} | From: {:?} | Amount: {} | Tokens: {} | Market: {} | Error: {}",
        tx_hash, result.from_address, format_eth(result.eth_amount), 
        tokens, market_type, error
    );
}

