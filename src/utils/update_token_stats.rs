use anyhow::Result; 
//use anyhow::Ok;
use sqlx::postgres::PgPool;
use log::{info, error, warn};
use alloy::primitives::Address;
use alloy_provider::ProviderBuilder;
use std::str::FromStr;
use std::env;
use crate::utils::misc::{list_registered_tokens, get_registered_tokens_count};

/// Get market cap for a token by calling the token contract for supply and price
async fn get_token_market_cap(
    token_address: &str,
    pool_address: &str,
) -> Result<Option<sqlx::types::BigDecimal>> {
    // Parse addresses
    let token_addr = match Address::from_str(token_address) {
        Ok(addr) => addr,
        Err(e) => {
            error!("Invalid token address format: {} - {}", token_address, e);
            return Ok(None);
        }
    };

    // Get RPC URL
    let rpc_url = env::var("MONAD_TESTNET_RPC_URL")?;
    let provider = ProviderBuilder::new().on_http(rpc_url.parse()?);

    // 1. Get circulating supply from token contract (totalSupply)
    let circulating_supply = get_token_total_supply(&provider, &token_addr).await?;
    if circulating_supply.is_none() {
        return Ok(None);
    }
    let circulating_supply = circulating_supply.unwrap();

    // 2. All tokens in our hardcoded mapping are graduated (have pools)
    // 3. Get token price from Uniswap pool
    let price_per_token = get_graduated_token_price(&provider, pool_address, token_address).await?;
    if price_per_token.is_none() {
        return Ok(None);
    }
    let price_per_token = price_per_token.unwrap();

    // 4. Calculate market cap = price * circulating supply (rounded to whole number)
    let market_cap_raw = &price_per_token * &circulating_supply;
    
    // Round to whole number by converting to string and parsing back without decimals
    let market_cap = sqlx::types::BigDecimal::from_str(&format!("{:.0}", market_cap_raw))?;
    
    info!("Market cap calculated for token {}: {} (price: {}, supply: {})", 
          token_address, market_cap, price_per_token, circulating_supply);
    
    Ok(Some(market_cap))
}

/// Test function to verify market cap calculation works
pub async fn test_market_cap_calculation(
    token_address: &str,
    pool_address: &str,
) -> Result<Option<sqlx::types::BigDecimal>> {
    get_token_market_cap(token_address, pool_address).await
}

/// Get total supply from token contract (circulating supply)
async fn get_token_total_supply<P>(
    provider: &P,
    token_address: &Address,
) -> Result<Option<sqlx::types::BigDecimal>>
where
    P: alloy_provider::Provider + Clone,
{
    // ERC20 totalSupply ABI
    let total_supply_abi = serde_json::json!([
        {
            "inputs": [],
            "name": "totalSupply",
            "outputs": [
                {
                    "internalType": "uint256",
                    "name": "",
                    "type": "uint256"
                }
            ],
            "stateMutability": "view",
            "type": "function"
        }
    ]);

    let json_abi: alloy::json_abi::JsonAbi = serde_json::from_value(total_supply_abi)?;
    let contract = alloy::contract::ContractInstance::new(
        *token_address,
        provider,
        alloy::contract::Interface::new(json_abi)
    );

    match contract.function("totalSupply", &[]) {
        Ok(call) => {
            match call.call().await {
                Ok(result) => {
                    if let Some(alloy::dyn_abi::DynSolValue::Uint(supply, _)) = result.first() {
                        sqlx::types::BigDecimal::from_str(&supply.to_string())
                            .map(|supply_bd| {
                                info!("Successfully fetched total supply: {}", supply_bd);
                                Some(supply_bd)
                            })
                            .map_err(|e| {
                                error!("Failed to parse total supply to BigDecimal: {}", e);
                                e
                            })
                            .map(Ok)
                            .unwrap_or_else(|_| Ok(None))
                    } else {
                        error!("Unexpected result type from totalSupply call");
                        Ok(None)
                    }
                }
                Err(e) => {
                    error!("Failed to call totalSupply on token {}: {}", token_address, e);
                    Ok(None)
                }
            }
        }
        Err(e) => {
            error!("Failed to create totalSupply function call: {}", e);
            Ok(None)
        }
    }
}

/// Get price for graduated tokens using QuoterV2
async fn get_graduated_token_price<P>(
    provider: &P,
    _pool_address: &str,
    token_address: &str,
) -> Result<Option<sqlx::types::BigDecimal>>
where
    P: alloy_provider::Provider + Clone,
{
    let token_addr = Address::from_str(token_address)?;
    
    // QuoterV2 contract address
    let quoter_address = Address::from_str("0x1b4e313fef15630af3e6f2de550dbf4cc9d3081d")?;
    
    // WMON address (WETH equivalent on Monad)
    let wmon_address = Address::from_str("0x760AfE86e5de5fa0Ee542fc7B7B713e1c5425701")?;
    
    // Load QuoterV2 ABI from file
    let quoter_abi_json = include_str!("../../hardhat/abi/QuoterV2.json");
    let quoter_abi_value: serde_json::Value = serde_json::from_str(quoter_abi_json)?;
    let quoter_abi = quoter_abi_value.get("abi")
        .ok_or_else(|| anyhow::anyhow!("Missing 'abi' field in QuoterV2.json"))?;

    let json_abi: alloy::json_abi::JsonAbi = serde_json::from_value(quoter_abi.clone())?;
    let contract = alloy::contract::ContractInstance::new(
        quoter_address,
        provider,
        alloy::contract::Interface::new(json_abi)
    );

    // Standard quote amount: 1 ETH (10^18 wei)
    let standard_quote_amount = alloy::primitives::Uint::from(10u64.pow(18));
    
    // Fee: 10000 (1%)
    let fee = alloy::primitives::Uint::from(10000u32);
    
    // sqrtPriceLimitX96: 0 (no limit)
    let sqrt_price_limit = alloy::primitives::Uint::from(0u32);

    // Create the params tuple for quoteExactInputSingle
    let params = alloy::dyn_abi::DynSolValue::Tuple(vec![
        alloy::dyn_abi::DynSolValue::Address(wmon_address),      // tokenIn (WMON)
        alloy::dyn_abi::DynSolValue::Address(token_addr),         // tokenOut (token)
        alloy::dyn_abi::DynSolValue::Uint(standard_quote_amount, 256), // amountIn (1 ETH)
        alloy::dyn_abi::DynSolValue::Uint(fee, 24),              // fee (10000)
        alloy::dyn_abi::DynSolValue::Uint(sqrt_price_limit, 160), // sqrtPriceLimitX96 (0)
    ]);

    match contract.function("quoteExactInputSingle", &[params]) {
        Ok(call) => {
            match call.call().await {
                Ok(result) => {
                    info!("QuoterV2 raw result for token {}: {:?}", token_address, result);
                    
                    // The result could be in different formats:
                    // 1. A single Uint (just amountOut)
                    // 2. A Tuple with multiple values
                    // 3. Multiple separate values
                    
                    let amount_out = if let Some(alloy::dyn_abi::DynSolValue::Uint(amount, _)) = result.first() {
                        // Case 1: Direct Uint value
                        Some(*amount)
                    } else if let Some(alloy::dyn_abi::DynSolValue::Tuple(tuple)) = result.first() {
                        // Case 2: Tuple containing (amountOut, sqrtPriceX96After, initializedTicksCrossed, gasEstimate)
                        if let Some(alloy::dyn_abi::DynSolValue::Uint(amount, _)) = tuple.first() {
                            Some(*amount)
                        } else {
                            None
                        }
                    } else if result.len() >= 4 {
                        // Case 3: Multiple return values as separate items
                        if let Some(alloy::dyn_abi::DynSolValue::Uint(amount, _)) = result.get(0) {
                            Some(*amount)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    
                    match amount_out {
                        Some(amount) if amount.to_string() != "0" => {
                            // Calculate price: 1 ETH / tokens_out = price per token in ETH
                            let eth_amount = sqlx::types::BigDecimal::from_str("1000000000000000000")?; // 1 ETH in wei
                            let tokens_received = sqlx::types::BigDecimal::from_str(&amount.to_string())?;
                            
                            let price_per_token = &eth_amount / &tokens_received;
                            
                            info!("QuoterV2 price for token {}: {} ETH per token (got {} tokens for 1 ETH)", 
                                  token_address, price_per_token, tokens_received);
                            Ok(Some(price_per_token))
                        }
                        Some(_) => {
                            warn!("QuoterV2 returned 0 tokens for token {}", token_address);
                            Ok(None)
                        }
                        None => {
                            error!("Could not extract amountOut from result. Result format: {:?}", result);
                            Ok(None)
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to call quoteExactInputSingle for token {}: {}", token_address, e);
                    Ok(None)
                }
            }
        }
        Err(e) => {
            error!("Failed to create quoteExactInputSingle function call: {}", e);
            Ok(None)
        }
    }
}

//////////////////////////////////
/// UPDATE DIAMOND HAND ACCOUNTS ///
/// //////////////////////////////////
#[derive(Debug, serde::Serialize)]
pub struct UserReputationInfo {
    pub diamond_hand_score: i32,    // 0-500 range (renamed from reputation_score)
    pub reputation_score: i32,      // 0-100k range (new field)
    pub global_rank: u64,
    pub avg_duration_z: f64,        // Average z-scores across all tokens
    pub avg_pnl_z: f64,
    pub avg_value_z: f64, 
    pub avg_volume_z: f64,
    pub avg_buy_volume_z: f64,      // Average buy volume z-score
    pub avg_sell_volume_z: f64,     // Average sell volume z-score
    pub total_holdings_value: f64,  // Total USD value
    pub total_volume: f64,          // Total trading volume
    pub total_buy_volume: f64,      // Total buy volume
    pub total_sell_volume: f64,     // Total sell volume
    pub total_pnl: f64,             // Total profit/loss
    pub avg_holding_duration: f64,  // Average holding duration
    pub tokens_held: u32,           // Number of different tokens
}

/// Optimized batch update z-scores for all tokens in a single query
pub async fn batch_update_z_scores_optimized(
    pool: &PgPool,
    limit: Option<usize>
) -> Result<u64, anyhow::Error> {
    
    let mut tx = pool.begin().await?;

    // Use hardcoded token contracts mapping instead of database query
    let registered_tokens = list_registered_tokens();
    info!("Updating market cap for {} hardcoded tokens", get_registered_tokens_count());

    // Update market cap for each token in the hardcoded mapping
    for (token_id, _, pool_address) in registered_tokens.iter() {
        match get_token_market_cap(token_id, pool_address).await {
            Ok(Some(market_cap)) => {
                // Update market cap in database (only if column exists)
                match sqlx::query!(
                    "UPDATE token_stats SET market_cap = $1 WHERE token_id = $2",
                    market_cap,
                    token_id
                )
                .execute(&mut *tx)
                .await
                {
                    Ok(_) => {
                        info!("Updated market cap for token {}: {}", token_id, market_cap);
                    }
                    Err(e) => {
                        // If column doesn't exist, just log the calculated value
                        if e.to_string().contains("column \"market_cap\" of relation \"token_stats\" does not exist") {
                            info!("Market cap calculated for token {}: {} (column not yet added to database)", token_id, market_cap);
                        } else {
                            error!("Failed to update market cap for token {}: {}", token_id, e);
                        }
                    }
                }
            }
            Ok(None) => {
                warn!("Could not calculate market cap for token {}", token_id);
            }
            Err(e) => {
                error!("Error calculating market cap for token {}: {}", token_id, e);
            }
        }
    }

    // 1. Batch update all token_stats in one query
    let stats_updated = sqlx::query!(
        r#"
        UPDATE token_stats
        SET
            mean_duration = stats.mean_duration,
            stddev_duration = stats.stddev_duration,
            mean_pnl = stats.mean_pnl,
            stddev_pnl = stats.stddev_pnl,
            mean_value = stats.mean_value,
            stddev_value = stats.stddev_value,
            mean_volume = stats.mean_volume,
            stddev_volume = stats.stddev_volume,
            mean_buy_volume = stats.mean_buy_volume,
            stddev_buy_volume = stats.stddev_buy_volume,
            mean_sell_volume = stats.mean_sell_volume,
            stddev_sell_volume = stats.stddev_sell_volume,
            liquidity_score = stats.liquidity_score,
            holder_weight = stats.holder_weight
        FROM (
            SELECT
                tb.token_id,
                COALESCE(AVG(
                    CASE 
                        WHEN tb.holding_duration > 0 THEN tb.holding_duration
                        ELSE EXTRACT(EPOCH FROM (NOW() - tb.first_bought))
                    END
                ), 0) as mean_duration,
                COALESCE(NULLIF(STDDEV(
                    CASE 
                        WHEN tb.holding_duration > 0 THEN tb.holding_duration
                        ELSE EXTRACT(EPOCH FROM (NOW() - tb.first_bought))
                    END
                ), 0), 1) as stddev_duration,
                COALESCE(AVG(tb.pnl), 0) as mean_pnl,
                COALESCE(NULLIF(STDDEV(tb.pnl), 0), 1) as stddev_pnl,
                COALESCE(AVG(tb.holdings_value), 0) as mean_value,
                COALESCE(NULLIF(STDDEV(tb.holdings_value), 0), 1) as stddev_value,
                COALESCE(AVG(tb.volume), 0) as mean_volume,
                COALESCE(NULLIF(STDDEV(tb.volume), 0), 1) as stddev_volume,
                COALESCE(AVG(tb.buy_volume), 0) as mean_buy_volume,
                COALESCE(NULLIF(STDDEV(tb.buy_volume), 0), 1) as stddev_buy_volume,
                COALESCE(AVG(tb.sell_volume), 0) as mean_sell_volume,
                COALESCE(NULLIF(STDDEV(tb.sell_volume), 0), 1) as stddev_sell_volume,
                COALESCE(LOG(SUM(tb.volume) + 1), 0) as liquidity_score,
                COALESCE(LOG(COUNT(*) + 1), 1) as holder_weight
            FROM token_balance tb
            WHERE tb.volume > 0 
                AND (tb.holding_duration > 600 OR EXTRACT(EPOCH FROM (NOW() - tb.first_bought)) > 600)
            GROUP BY tb.token_id
            LIMIT $1
        ) as stats
        WHERE token_stats.token_id = stats.token_id
        "#,
        limit.map(|l| l as i64).unwrap_or(i64::MAX)
    )
    .execute(&mut *tx)
    .await?;

    // 2. Batch update all z-scores in one query
    let z_scores_updated = sqlx::query!(
        r#"
        UPDATE token_balance 
        SET
            duration_z = (
                CASE 
                    WHEN token_balance.holding_duration > 0 THEN token_balance.holding_duration
                    ELSE EXTRACT(EPOCH FROM (NOW() - token_balance.first_bought))
                END - ts.mean_duration
            ) / ts.stddev_duration,
            pnl_z = (token_balance.pnl - ts.mean_pnl) / ts.stddev_pnl,
            value_z = (token_balance.holdings_value - ts.mean_value) / ts.stddev_value,
            volume_z = (token_balance.volume - ts.mean_volume) / ts.stddev_volume,
            buy_volume_z = (token_balance.buy_volume - ts.mean_buy_volume) / ts.stddev_buy_volume,
            sell_volume_z = (token_balance.sell_volume - ts.mean_sell_volume) / ts.stddev_sell_volume
        FROM token_stats ts
        WHERE token_balance.token_id = ts.token_id 
            AND token_balance.holdings_value > 0
            AND (token_balance.holding_duration > 600 OR EXTRACT(EPOCH FROM (NOW() - token_balance.first_bought)) > 600)
        "#
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    
    info!("Batch updated stats for {} tokens and z-scores for {} records", 
          stats_updated.rows_affected(), z_scores_updated.rows_affected());
    
    Ok(z_scores_updated.rows_affected())
}

/// Optimized batch reputation calculation for all users in one query
/// Now uses market_cap as a weighting factor instead of holder_weight
/// Uses sqrt(sqrt(market_cap)) to give more weight to higher market caps without being too skewed
pub async fn batch_calculate_user_reputation_optimized(
    pool: &PgPool,
    limit: Option<usize>
) -> Result<u64, anyhow::Error> {
    
    let updated = sqlx::query!(
        r#"
        UPDATE account 
        SET 
            diamond_hand_probability = LEAST(500, GREATEST(0, 
                ((user_scores.weighted_z_score * LN(user_scores.total_weight + 1) + 5.0) * 50.0)::int
            )),
            reputation = LEAST(100000, GREATEST(0,
                ((user_scores.weighted_z_score * LN(user_scores.total_weight + 1) + 5.0) * 10000.0)::int
            ))
        FROM (
            SELECT 
                tb.account_id,
                COALESCE(SUM(
                    (tb.duration_z * 0.45 + 
                     tb.value_z * 0.45 + 
                     tb.buy_volume_z * 0.09 + 
                     tb.sell_volume_z * 0.01) * 
                    POWER(COALESCE(ts.market_cap, 1), 0.15)
                ) / NULLIF(SUM(POWER(COALESCE(ts.market_cap, 1), 0.15)), 0), 0) as weighted_z_score,
                COALESCE(LN(SUM(POWER(COALESCE(ts.market_cap, 1), 0.15)) + 1), 1) as total_weight
            FROM token_balance tb
            JOIN token_stats ts ON tb.token_id = ts.token_id
            WHERE tb.holdings_value > 0
            GROUP BY tb.account_id
            LIMIT $1
        ) as user_scores
        WHERE account.id = user_scores.account_id
            AND account.slug IS NULL
        "#,
        limit.map(|l| l as i64).unwrap_or(i64::MAX)
    )
    .execute(pool)
    .await?;

    // Reset reputation for users with no holdings
    let reset = sqlx::query!(
        r#"
        UPDATE account
        SET 
            diamond_hand_probability = 0,
            reputation = 0
        WHERE 
            slug IS NULL 
            AND id NOT IN (SELECT DISTINCT account_id FROM token_balance WHERE holdings_value > 0)
        "#
    )
    .execute(pool)
    .await?;

    info!("Batch updated reputation for {} users and reset reputation for {} users", updated.rows_affected(), reset.rows_affected());
    Ok(updated.rows_affected() + reset.rows_affected())
}

/// Optimized sequential execution ensuring correct data dependencies
pub async fn sequential_update_all_stats_optimized(
    pool: &PgPool,
    token_limit: Option<usize>,
    user_limit: Option<usize>
) -> Result<(u64, u64), anyhow::Error> {
    
    let start_time = std::time::Instant::now();
    
    // MUST run sequentially due to data dependencies:
    // 1. First update z-scores (token_stats + token_balance z-scores)
    let z_score_updated = batch_update_z_scores_optimized(pool, token_limit).await?;
    
    // 2. Then update reputation using the fresh z-scores
    let users_updated = batch_calculate_user_reputation_optimized(pool, user_limit).await?;
    
    let duration = start_time.elapsed();
    info!("Sequential optimized stats update completed in {:?} - Z-scores: {}, Users: {}", 
          duration, z_score_updated, users_updated);
    
    Ok((z_score_updated, users_updated))
}
