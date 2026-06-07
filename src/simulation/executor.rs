use crate::simulation::models::{MarketType, SimulationStats, TransactionResult};
use crate::simulation::utils::{calculate_fee, format_eth, random_eth_amount, should_include_referral};
use anyhow::{anyhow, Result};
use ethers::{
    prelude::*,
    types::{H160, U256},
};
use log::{debug, error, info, warn};
use rand::Rng;
use std::sync::Arc;
use tokio::sync::Mutex;

// ABI for the Cult token contract
const CULT_ABI: &str = r#"[
    {
        "inputs": [
            {"name": "recipient", "type": "address"},
            {"name": "refundRecipient", "type": "address"},
            {"name": "orderReferrer", "type": "address"},
            {"name": "comment", "type": "string"},
            {"name": "expectedMarketType", "type": "uint8"},
            {"name": "minOrderSize", "type": "uint256"},
            {"name": "sqrtPriceLimitX96", "type": "uint160"}
        ],
        "name": "buy",
        "outputs": [{"name": "", "type": "uint256"}],
        "stateMutability": "payable",
        "type": "function"
    },
    {
        "inputs": [],
        "name": "marketType",
        "outputs": [{"name": "", "type": "uint8"}],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [{"name": "ethOrderSize", "type": "uint256"}],
        "name": "getEthBuyQuote",
        "outputs": [{"name": "", "type": "uint256"}],
        "stateMutability": "view",
        "type": "function"
    }
]"#;

/// Parameters for a buy operation
pub struct BuyParams {
    pub eth_amount: U256,
    pub recipient: H160,
    pub refund_recipient: H160,
    pub order_referrer: Option<H160>,
    pub comment: String,
    pub market_type: MarketType,
    pub min_order_size: U256,
}

/// Execute a single buy transaction
pub async fn execute_buy_transaction(
    contract: &Contract<Provider<Http>>,
    wallet: &LocalWallet,
    params: BuyParams,
    stats: Arc<Mutex<SimulationStats>>,
) -> Result<TransactionResult> {
    // Default sqrtPriceLimitX96 to 0 (ignored for BONDING_CURVE)
    let sqrt_price_limit_x96 = U256::zero();
    
    let mut result = TransactionResult {
        tx_hash: None,
        from_address: wallet.address(),
        to_address: contract.address(),
        eth_amount: params.eth_amount,
        tokens_received: None,
        fee_paid: calculate_fee(params.eth_amount, 100), // 1% fee
        success: false,
        error: None,
        market_type: params.market_type,
    };
    
    // Connect wallet to provider
    let provider = contract.client();
    let chain_id = provider.get_chainid().await?;
    let wallet_with_provider = wallet.clone().with_chain_id(chain_id.as_u64()).connect(provider.clone());
    
    // Create the contract instance with signer
    let contract_with_signer = contract.connect(wallet_with_provider);
    
    debug!("Sending buy transaction from {}, amount: {}", 
           wallet.address(), format_eth(params.eth_amount));
    
    // Prepare buy call
    let order_referrer = params.order_referrer.unwrap_or(H160::zero());
    let call = contract_with_signer.method(
        "buy", 
        (
            params.recipient,
            params.refund_recipient,
            order_referrer,
            params.comment,
            params.market_type as u8,
            params.min_order_size,
            sqrt_price_limit_x96,
        )
    )?;
    
    // Send transaction with value
    match call.value(params.eth_amount).send().await {
        Ok(tx) => {
            result.tx_hash = Some(tx.tx_hash());
            info!("Transaction sent: {:?}", tx.tx_hash());
            
            // Wait for transaction receipt
            match tx.await {
                Ok(receipt) => {
                    if let Some(status) = receipt.status {
                        if status.as_u64() == 1 {
                            // Parse logs or events to find tokens received
                            // This is a simplified approach - in production you would parse the specific event
                            let tokens_received = match receipt.logs.first() {
                                Some(log) if !log.data.0.is_empty() => {
                                    // Example parsing of token amount from log data
                                    // In reality, you would decode the specific event
                                    let data = &log.data.0;
                                    if data.len() >= 32 {
                                        let amount_bytes = &data[data.len() - 32..];
                                        Some(U256::from_big_endian(amount_bytes))
                                    } else {
                                        // Fallback to a default value
                                        Some(params.eth_amount * U256::from(100))
                                    }
                                }
                                _ => Some(params.eth_amount * U256::from(100)), // Approximation
                            };
                            
                            result.success = true;
                            result.tokens_received = tokens_received;
                            
                            // Update global stats
                            let mut stats = stats.lock().await;
                            stats.total_transactions += 1;
                            stats.successful_transactions += 1;
                            stats.total_eth_spent += params.eth_amount;
                            stats.total_fees_paid += result.fee_paid;
                            if let Some(tokens) = tokens_received {
                                stats.total_tokens_received += tokens;
                            }
                            
                            info!("Transaction successful: {:?} tokens purchased for {}",
                                  tokens_received.map(format_eth), format_eth(params.eth_amount));
                        } else {
                            result.error = Some("Transaction reverted".to_string());
                            
                            // Update global stats
                            let mut stats = stats.lock().await;
                            stats.total_transactions += 1;
                            stats.failed_transactions += 1;
                            
                            

