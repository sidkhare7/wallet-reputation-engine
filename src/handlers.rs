use anyhow::Ok;
use anyhow::Result; 
use sqlx::{Transaction, Postgres}; 
use reqwest;
use chrono::{DateTime, TimeZone, Utc}; // For handling timestamps
use sqlx::types::BigDecimal;
use std::str::FromStr;                  // for parsing string -> BigDecimal
use serde::Deserialize;
use serde::de::{self, Deserializer};
//use crate::utils::update_token_stats::update_user_token_z_scores; 
use log::{error, info};
use std::result::Result as StdResult;
use crate::utils::misc::{print_token_contracts, get_token_contracts, is_contract_address_fallback, get_token_id_by_pool_address, get_transaction_from_address_fallback, get_token_balance_from_contract};

/// Helper function to determine if holding duration should be updated
/// Returns true if:
/// 1. Sell/transfer amount is > 5% of current balance, OR
/// 2. Last transfer was within 1 hour
fn should_update_holding_duration(
    transfer_amount: &BigDecimal,
    current_balance: &BigDecimal,
    last_updated: &chrono::DateTime<chrono::Utc>,
) -> Result<bool> {
    // Check if transfer amount is > 20% of current balance
    let five_percent_threshold = current_balance * BigDecimal::from_str("0.20")?;
    if transfer_amount > &five_percent_threshold {
        return Ok(true);
    }
    
    // Check if last transfer was within 1 hour
    let one_hour_ago = chrono::Utc::now() - chrono::Duration::hours(1);
    let recent_transfer = last_updated > &one_hour_ago;
    
    Ok(recent_transfer)
}

// WETH address on Monad Testnet (from deployment script)
const WETH_ADDRESS: &str = "0x760AfE86e5de5fa0Ee542fc7B7B713e1c5425701";

// Introduce 'tx for the Transaction's own lifetime, 'a for the borrow lifetime
pub async fn create_account<'tx, 'a>(
    // The transaction itself is valid for 'tx (tied to its connection)
    // We borrow it mutably for 'a within this function
    tx: &'a mut Transaction<'tx, Postgres>,
    address: &'a str,
    slug: Option<&'a str>,
    referrer_id: Option<&'a str>,
) -> Result<bool>
where
    'tx: 'a, // Transaction must live at least as long as the borrow
{    
    //Check if the address is a contract - if so, don't create an account
    if is_contract_address_fallback(address).await? {
        println!("Skipping account creation for contract address: {}", address);
        return Ok(false);
    }
    
    let referral_code: Option<String> = None;
    let diamond_hand_probability: i32 = 0;
    let total_referrals: i32 = 0;
    let fee_collected: f64 = 0.0;
    let twitter: Option<&str> = None;
    let discord: Option<&str> = None;
    
    sqlx::query(
        r#"
        INSERT INTO account (id, slug, referral_code, diamond_hand_probability, referrer_id, total_referrals, fee_collected, twitter, discord, is_registered)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (id) DO NOTHING
        "#
    )
    .bind(address)
    .bind(slug)
    .bind(referral_code)
    .bind(diamond_hand_probability)
    .bind(referrer_id)
    .bind(total_referrals)
    .bind(fee_collected)
    .bind(twitter)
    .bind(discord)
    .bind(false)
    // Pass `tx` directly. `&'a mut Transaction<'tx, Pg>` implements Executor<'a, Pg>.
    .execute(&mut **tx).await?;

    println!("Successfully created/updated account for address: {}", address);
    Ok(true)
}

pub async fn handle_cult_token_created(
    event: CultTokenCreatedEvent,
    tx: &mut sqlx::Transaction<'_, Postgres>
) -> Result<(), anyhow::Error> {
    
    println!("Processing CultTokenCreated event for token: {:?}", event);
    let mut ipfs_content = "".to_string();
    // Handle IPFS data
    if event.token_uri.starts_with("ipfs://") {
        let hash = event.token_uri.trim_start_matches("ipfs://").to_string();
        let ipfs_url = format!("https://ipfs.io/ipfs/{}", hash);

        let client = reqwest::Client::new();
        match client.get(&ipfs_url).send().await {
            StdResult::Ok(response) => {
                if response.status().is_success() {
                     match response.text().await {
                         StdResult::Ok(text) => ipfs_content = text,
                         Err(e) => println!("Failed to read IPFS text content for token {}: {}", event.token_address, e), // Log error, continue
                     }
                } else {
                    println!("Failed IPFS fetch for token {}, status: {}", event.token_address, response.status()); // Log error, continue
                }
            }
            Err(e) => {
                println!("Failed to send IPFS request for token {}: {}", event.token_address, e); // Log error, continue
            }
        }
    }

    // --- Type Conversions ---
    // Convert block_number to i64 for BIGINT compatibility
    let block_number_db: i64 = match event.block_number.try_into() {
        std::result::Result::Ok(bn) => bn,
        Err(_) => {
            error!("Block number {} too large to fit in i64 for token {}", event.block_number, event.token_address);
            return Err(anyhow::anyhow!("Block number {} too large to fit in i64", event.block_number));
        }
   };

   let total_airdrop_recipient_count_db: i64 = match event.total_airdrop_recipient_count.try_into() {
            std::result::Result::Ok(ta) => ta,
            Err(_) => {
                error!("total_airdrop_recipient_count {} too large to fit in i64 for token {}", 
            event.total_airdrop_recipient_count, event.token_address);
                return Err(anyhow::anyhow!("total_airdrop_recipient_count {} too large to fit in i64", 
            event.total_airdrop_recipient_count));
            }
    };

    let total_amount = BigDecimal::from_str(&event.total_amount.to_string())?;
    

   // Convert u64 Unix timestamp to DateTime<Utc> for TIMESTAMPTZ
   let block_timestamp_db: DateTime<Utc> = Utc.timestamp_opt(event.block_timestamp as i64, 0).single()
       .ok_or_else(|| {
           error!("Invalid block timestamp {} for token {}", event.block_timestamp, event.token_address);
           anyhow::anyhow!("Invalid block timestamp: {}", event.block_timestamp)
       })?;


    println!("Processing insert into cult_token");
    // Insert or update the cult token - using upsert to prevent duplicates
    sqlx::query(
        r#"
        INSERT INTO cult_token (
            id, name, symbol, pool_address, block_number,
            block_timestamp, transaction_hash, airdrop_contract, 
            total_amount, total_airdrop_recipient_count, ipfs_content,
            lpPositionId, volume
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ON CONFLICT (id) DO NOTHING
        "#
    )
    .bind(&event.token_address)
    .bind(&event.name)
    .bind(&event.symbol)
    .bind(&event.pool_address)
    .bind(block_number_db)
    .bind(block_timestamp_db)
    .bind(&event.transaction_hash)
    .bind(&event.airdrop_contract)
    .bind(&total_amount)
    .bind(total_airdrop_recipient_count_db)
    .bind(ipfs_content)
    .bind(0i64) // lpPositionId default
    .bind(BigDecimal::from(0)) // volume default
    .execute(&mut **tx).await?;

    // Create accounts
    create_account(&mut *tx, &event.pool_address, Some("POOL"), None).await?;
    // not needed as creators will always have account for now
    //create_account(&mut *tx, &event.token_creator, Some("CREATOR"), None).await?;
    create_account(&mut *tx, &event.airdrop_contract, Some("AIRDROP"), None).await?;

    // Register token contracts in the hashmap for transfer handling
    print_token_contracts(&event.token_address, &event.airdrop_contract, &event.pool_address);
    println!("Registered token contracts for {}: airdrop={}, pool={}", 
        event.token_address, event.airdrop_contract, event.pool_address);

    println!("Initializing token_stats");
    // Initialize or update token_stats - using upsert to prevent duplicates
    sqlx::query(
        r#"
        INSERT INTO token_stats (
            token_id, mean_duration, stddev_duration, mean_pnl, stddev_pnl,
            mean_value, stddev_value, mean_volume, stddev_volume, mean_buy_volume, stddev_buy_volume,
            mean_sell_volume, stddev_sell_volume, liquidity_score, holder_weight
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        ON CONFLICT (token_id) DO NOTHING
        "#
    )
    .bind(&event.token_address)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .execute(&mut **tx).await?;
    
    // Flatten all merkle roots into a single vector
    let all_merkle_roots: Vec<&str> = event.merkle_roots
            .iter()
            .flat_map(|root_str| root_str.split(','))
            .map(|root| root.trim())
        .collect();

    println!("Processing {} individual merkle roots", all_merkle_roots.len());

    // Process each individual merkle root
    for merkle_root in all_merkle_roots {
        // Convert hex string (with or without 0x prefix) to Vec<u8>
        let merkle_root_bytes = hex::decode(merkle_root.trim_start_matches("0x"))
            .map_err(|e| anyhow::anyhow!("Invalid merkle root hex: {}, error: {}", merkle_root, e))?;
    
        // Find the community for this merkle root
        let community: Option<(String, String, serde_json::Value)> = sqlx::query_as(
            r#"
            SELECT id, name, merkle_proofs 
            FROM communities 
            WHERE merkle_root = $1
            "#
        )
        .bind(&merkle_root_bytes)
        .fetch_optional(&mut **tx)
        .await?;

        let (community_id, community_name, merkle_proofs) = match community {
            Some(c) => c,
            None => {
                eprintln!("No community found for merkle root: {:?}", merkle_root);
                continue; // Skip but continue processing other roots
            }
        };

        // Convert recipient count to i64
        let recipients_db: i64 = event.total_airdrop_recipient_count
            .try_into()
            .map_err(|_| anyhow::anyhow!("Recipient count exceeds i64 bounds"))?;

        // Insert into token_airdrops using event totals directly - ignore duplicates
        let token_airdrop_result = sqlx::query_as::<_, (i32,)>(
            r#"
            INSERT INTO token_airdrops (
                transaction_hash, 
                token_id, 
                merkle_root, 
                community_id, 
                community_name, 
                merkle_proofs, 
                total_amount, 
                total_recipient_count, 
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            ON CONFLICT DO NOTHING
            RETURNING id
            "#
        )
        .bind(&event.transaction_hash)
        .bind(&event.token_address)
        .bind(&merkle_root_bytes)
        .bind(&community_id)
        .bind(&community_name)
        .bind(&merkle_proofs)
        .bind(&total_amount) // Use total directly from event
        .bind(recipients_db)       // Use total directly from event
        .fetch_optional(&mut **tx)
        .await?;

        let merkle_proofs_obj = merkle_proofs
            .as_object()
            .ok_or(anyhow::anyhow!("Invalid merkle_proofs format"))?;

        let recipient_addresses: Vec<&str> = merkle_proofs_obj.keys().map(|k| k.as_str()).collect();

        // Only proceed if we actually inserted a new record (not a duplicate)
        if let Some((token_airdrop_id,)) = token_airdrop_result {
            // Use a more efficient batch insert approach
            if !recipient_addresses.is_empty() {
            let placeholders: Vec<String> = recipient_addresses.iter()
                .enumerate()
                .map(|(i, _)| format!("(${}, ${})", i * 2 + 1, i * 2 + 2))
                .collect();
            
            let query = format!(
                "INSERT INTO airdrop_recipients (account_id, token_airdrop_id) VALUES {} ON CONFLICT DO NOTHING",
                placeholders.join(", ")
            );
            
            let mut sql_query = sqlx::query(&query);
            
            // Bind all the values
            for address in &recipient_addresses {
                sql_query = sql_query.bind(address).bind(token_airdrop_id);
            }
            
            sql_query.execute(&mut **tx).await?;
        }
        }
    }
    
    println!("Successfully processed CultTokenCreated event for token {}", 
        event.token_address);
    Ok(())
}

//TODO ignore transfer from pool contract or any conracts (this is to avoid reading this as trade)
pub async fn handle_cult_token_transfer(
    event: CultTokenTransferEvent,
    tx: &mut sqlx::Transaction<'_, Postgres>
) -> Result<(), anyhow::Error> { 
    println!("Processing CultTokenTransfer event for token: {:?}", event);

    if event.from == event.token_id {
        println!("Ignoring transfer it's graduation event: {}", event.transaction_hash);
        return Ok(());
    }

    if event.from == "0x0000000000000000000000000000000000000000" {
        println!("Ignoring transfer from zero address: {}", event.transaction_hash);
        return Ok(());
    }

    // Convert balances once at the start
    let from_balance_bd = BigDecimal::from_str(&event.from_token_balance.to_string())?;
    let to_balance_bd = BigDecimal::from_str(&event.to_token_balance.to_string())?;

    // Get contracts for this token
    if let Some((airdrop_contract, pool_contract)) = get_token_contracts(&event.token_id) {
        // Check if from address is pool contract - ignore if so (swap handler will handle)
        if event.from == pool_contract {
            println!("Ignoring transfer from pool contract for token: {}", event.token_id);
            return Ok(());
        }

        // Check if from address is airdrop contract
        if event.from == airdrop_contract {
            // Handle as a claim event
            println!("Detected airdrop contract transfer, handling as claim event");
            //creating account for airdrop claimer if it doesn't exist
            let claimer_account_created = create_account(&mut *tx, &event.to, None, None).await?;
            if !claimer_account_created {
                println!("Skipping airdrop claim processing for contract address: {}", event.to);
                return Ok(());
            }
            // Mark airdrop as claimed
            sqlx::query!(
                r#"
                UPDATE airdrop_recipients
                SET claimed = TRUE
                WHERE account_id = $1 AND token_airdrop_id IN (
                    SELECT id FROM token_airdrops WHERE token_id = $2
                )
                "#,
                event.to,
                event.token_id
            )
            .execute(&mut **tx)
            .await?;

            // Handle recipient balance as claimed token
            sqlx::query(
                r#"
                INSERT INTO token_balance (
                    account_id, token_id, first_bought, volume, buy_volume, sell_volume, holding_duration,
                    pnl, holdings_value, duration_z, pnl_z, value_z, cost_basis, last_updated
                )
                VALUES ($1, $2, NOW(), 0, 0, 0, 0, 0, $3, 0, 0, 0, 0, NOW())
                ON CONFLICT (account_id, token_id)
                DO UPDATE SET
                    holdings_value = EXCLUDED.holdings_value,
                    first_bought = COALESCE(token_balance.first_bought, NOW()),
                    cost_basis = CASE
                        WHEN token_balance.holdings_value = 0 THEN 0
                        ELSE 
                            (token_balance.cost_basis * token_balance.holdings_value + 
                            0 * (EXCLUDED.holdings_value - token_balance.holdings_value)) / 
                            EXCLUDED.holdings_value
                    END,
                    last_updated = NOW()
                "#
            )
            .bind(&event.to)
            .bind(&event.token_id)
            .bind(&to_balance_bd)
            .execute(&mut **tx)
            .await
            .map_err(|e| {
                error!("Failed to update recipient balance for {}: {}", event.to, e);
                anyhow::anyhow!("Database error during recipient balance update: {}", e)
            })?;

            println!("Successfully processed token claim event for token {}", event.token_id);
            return Ok(());
        }
    } else {
        println!("No contract mapping found for token: {}, treating as regular transfer", event.token_id);
    }

    // Create accounts for both addresses first (contract check is handled inside create_account)
    let from_account_created = create_account(&mut *tx, &event.from, None, None).await?;
    let to_account_created = create_account(&mut *tx, &event.to, None, None).await?;
    

    if from_account_created {
    // Handle sender balance (delete if zero, update if not)
    if from_balance_bd == BigDecimal::from(0) {
        sqlx::query(
            r#"
            DELETE FROM token_balance 
            WHERE account_id = $1 AND token_id = $2 AND holdings_value > 0
            "#
        )
        .bind(&event.from)
        .bind(&event.token_id)
        .execute(&mut **tx)
        .await?;
    } else {
        // Update sender's balance
        sqlx::query(
            r#"
            INSERT INTO token_balance (
                account_id, token_id, first_bought, volume, buy_volume, sell_volume, holding_duration,
                pnl, holdings_value, duration_z, pnl_z, value_z, cost_basis, last_updated
            )
            VALUES ($1, $2, NOW(), 0, 0, 0, 0, 0, $3, 0, 0, 0, 0, NOW())
            ON CONFLICT (account_id, token_id)
            DO UPDATE SET
                holdings_value = EXCLUDED.holdings_value,
                first_bought = COALESCE(token_balance.first_bought, NOW()),
                holding_duration = CASE 
                    WHEN token_balance.holding_duration IS NULL OR token_balance.holding_duration = 0
                    THEN EXTRACT(EPOCH FROM (NOW() - token_balance.first_bought))
                    ELSE token_balance.holding_duration
                END,
                cost_basis = CASE 
                    WHEN EXCLUDED.holdings_value = 0 THEN 0
                    ELSE token_balance.cost_basis
                END,
                last_updated = NOW()
            "#
        )
        .bind(&event.from)
        .bind(&event.token_id)
        .bind(&from_balance_bd)
        .execute(&mut **tx)
        .await?;
    }

}
    if to_account_created {
    // Handle recipient balance
    sqlx::query(
        r#"
        INSERT INTO token_balance (
            account_id, token_id, first_bought, volume, buy_volume, sell_volume, holding_duration,
            pnl, holdings_value, duration_z, pnl_z, value_z, cost_basis, last_updated
        )
        VALUES ($1, $2, NOW(), 0, 0, 0, 0, 0, $3, 0, 0, 0, 0, NOW())
        ON CONFLICT (account_id, token_id)
        DO UPDATE SET
            holdings_value = EXCLUDED.holdings_value,
            first_bought = COALESCE(token_balance.first_bought, NOW()),
            cost_basis = CASE
                WHEN token_balance.holdings_value = 0 THEN 0
                ELSE 
                    (token_balance.cost_basis * token_balance.holdings_value + 
                     0 * (EXCLUDED.holdings_value - token_balance.holdings_value)) / 
                    EXCLUDED.holdings_value
            END,
            last_updated = NOW()
        "#
    )
    .bind(&event.to)
    .bind(&event.token_id)
    .bind(&to_balance_bd)
    .execute(&mut **tx)
    .await
    .map_err(|e| {
        error!("Failed to update recipient balance for {}: {}", event.to, e);
        anyhow::anyhow!("Database error during recipient balance update: {}", e)
    })?;
    }

    info!("=== CULT TOKEN TRANSFER EVENT COMPLETE in ===");
    Ok(())
}

pub async fn handle_cult_swap(
    event: CultSwapEvent,
    tx: &mut sqlx::Transaction<'_, Postgres>
) -> Result<(), anyhow::Error> {
    
    // Get the token_id from the pool_address using local hashmap
    let token_id = get_token_id_by_pool_address(&event.pool_address);
    println!("Processing CultTokenSwap event for token: {:?} , {:?}", token_id, event.transaction_hash);
    // Handle case where token is not found
    let token_id = match token_id {
        Some(id) => id,
        None => {
            println!("Token not found for pool address: {} - skipping swap", event.pool_address);
            return Ok(());
        }
    };

    // Determine token ordering based on lexicographic comparison (same as Uniswap V3)
    // In Uniswap V3, token0 is the lexicographically smaller address
    let weth_is_token0 = WETH_ADDRESS.to_lowercase() < token_id.to_lowercase();
    
    // Extract ETH and token amounts based on correct ordering
    let (eth_amount, token_amount) = if weth_is_token0 {
        // WETH is token0, CULT token is token1
        (event.amount0.abs(), event.amount1.abs())
    } else {
        // CULT token is token0, WETH is token1  
        (event.amount1.abs(), event.amount0.abs())
    };

    // Determine trade direction based on which amounts are positive/negative
    // Buy: ETH flows into pool (positive ETH delta), tokens flow out (negative token delta)
    // Sell: Tokens flow into pool (positive token delta), ETH flows out (negative ETH delta)
    let (eth_delta, token_delta) = if weth_is_token0 {
        (event.amount0.clone(), event.amount1.clone())
    } else {
        (event.amount1.clone(), event.amount0.clone())
    };

    let is_buy = eth_delta > BigDecimal::from(0) && token_delta < BigDecimal::from(0);
    let is_sell = token_delta > BigDecimal::from(0) && eth_delta < BigDecimal::from(0);

    if !is_buy && !is_sell {
        println!("Unable to determine trade direction for swap: eth_delta={}, token_delta={}", eth_delta, token_delta);
        return Ok(());
    }

    // Get the actual account that initiated the transaction by fetching from blockchain
    let tx_from_result = get_transaction_from_address_fallback(&event.transaction_hash).await;
    let trading_account = match tx_from_result {
        Result::Ok(Some(tx_from)) => {
            println!("Found actual transaction initiator: {}", tx_from);
            tx_from
        }
        Result::Ok(None) => {
            println!("Could not fetch transaction details, falling back to sender/recipient determination");
            if is_buy {
                event.recipient.clone()  // Recipient is buying tokens
            } else {
                event.sender.clone()     // Sender is selling tokens
            }
        }
        Result::Err(e) => {
            println!("Error fetching transaction details: {}, falling back to sender/recipient determination", e);
            if is_buy {
                event.recipient.clone()  // Recipient is buying tokens
            } else {
                event.sender.clone()     // Sender is selling tokens
            }
        }
    };

    // Create account for the actual trader
    let trader_account_created = create_account(&mut *tx, &trading_account, None, None).await?;
    if !trader_account_created {
        println!("Skipping swap processing for contract address: {}", trading_account);
        return Ok(());
    }

    println!("Token ordering: WETH is token0={}, is_buy={}", weth_is_token0, is_buy);
    println!("ETH amount: {}", eth_amount);
    println!("Token amount: {}", token_amount);

    // Guard against divide-by-zero
    if token_amount == BigDecimal::from(0) {
        println!("Token amount is zero – skipping swap record");
        return Ok(());
    }

    // Price (ETH per CULT token)
    let price_per_token = &eth_amount / &token_amount;

    // ------------------------------------------------------------------
    // Update token_balance for the trader
    // ------------------------------------------------------------------
    if is_buy {
        // BUY: User gains CULT tokens
        // First, check if record exists to see if we need to reset holding_duration
        let existing_balance = sqlx::query_as::<_, (Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>)>(
            r#"
            SELECT first_bought, last_updated
            FROM token_balance 
            WHERE account_id = $1 AND token_id = $2
            "#
        )
        .bind(&trading_account)
        .bind(&token_id)
        .fetch_optional(&mut **tx)
        .await?;

        // Check if time difference is over 2 weeks (14 days)
        let should_reset_duration = if let Some((first_bought, last_updated)) = existing_balance {
            if let (Some(first), Some(last)) = (first_bought, last_updated) {
                let time_diff = last.signed_duration_since(first);
                // 2 weeks = 14 days = 1,209,600 seconds
                time_diff.num_seconds() > 1_209_600
            } else {
                false
            }
        } else {
            false
        };

        sqlx::query(
            r#"
            INSERT INTO token_balance (
                account_id,
                token_id,
                first_bought,
                volume,
                buy_volume,
                sell_volume,
                holding_duration,
                pnl,
                cost_basis,
                holdings_value,
                duration_z,
                pnl_z,
                value_z,
                volume_z,
                buy_volume_z,
                sell_volume_z,
                last_updated
            )
            VALUES (
                $1, $2, NOW(), $3, $3, 0, 0, 0, $4, $5, 0, 0, 0, 0, 0, 0, NOW()
            )
            ON CONFLICT (account_id, token_id)
            DO UPDATE SET
                volume = token_balance.volume + $3,
                buy_volume = token_balance.buy_volume + $3,
                holdings_value = token_balance.holdings_value + $5,
                -- Calculate weighted average cost basis: (old_cost * old_holdings + new_cost * new_tokens) / total_holdings
                cost_basis = (
                    token_balance.cost_basis * token_balance.holdings_value + 
                    $4 * $5
                ) / (token_balance.holdings_value + $5),
                -- Update PnL: current_value - total_cost
                pnl = (token_balance.holdings_value + $5) * $4 - 
                      ((token_balance.cost_basis * token_balance.holdings_value + $4 * $5)),
                -- Reset holding_duration to 0 if time difference is over 2 weeks
                holding_duration = CASE 
                    WHEN $6 = true THEN 0
                    ELSE token_balance.holding_duration
                END,
                last_updated = NOW()
            "#
        )
        .bind(&trading_account)
        .bind(&token_id)
        .bind(&eth_amount)           // volume in ETH
        .bind(&price_per_token)      // cost_basis = current price per token
        .bind(&token_amount)         // holdings_value in tokens
        .bind(should_reset_duration) // should_reset_duration flag
        .execute(&mut **tx)
        .await?;

    } else {
        // SELL: User loses CULT tokens
        println!("Processing SELL transaction for account: {}", trading_account);
        
        // First, get current holdings to calculate sell ratio and update cost basis
        let current_balance = sqlx::query_as::<_, (BigDecimal, BigDecimal, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
            r#"
            SELECT holdings_value, cost_basis, first_bought, last_updated
            FROM token_balance 
            WHERE account_id = $1 AND token_id = $2
            "#
        )
        .bind(&trading_account)
        .bind(&token_id)
        .fetch_optional(&mut **tx)
        .await?;

        if let Some(balance) = current_balance {
            let current_holdings = balance.0;
            let current_cost_basis = balance.1;
            let last_updated = balance.3;
            
            // Don't allow selling more than they have
            let tokens_to_sell = token_amount.min(current_holdings.clone());
            
            // Check if we should update holding duration
            let should_update_duration = should_update_holding_duration(
                &tokens_to_sell, 
                &current_holdings,
                &last_updated
            )?;
            
            // Calculate PnL for this sell: (sell_price - cost_basis) * tokens_sold
            let sell_pnl = (&price_per_token - &current_cost_basis) * &tokens_to_sell;
            let new_holdings = &current_holdings - &tokens_to_sell;
            

            if new_holdings <= BigDecimal::from(0) {
                // Complete sell - set holding_duration and remove record
                sqlx::query(
                    r#"
                    UPDATE token_balance
                    SET
                        volume = volume + $3,
                        sell_volume = sell_volume + $3,
                        holdings_value = 0,
                        holding_duration = CASE 
                            WHEN $4 = true AND (token_balance.holding_duration IS NULL OR token_balance.holding_duration = 0)
                            THEN EXTRACT(EPOCH FROM (NOW() - token_balance.first_bought))
                            ELSE token_balance.holding_duration
                        END,
                        pnl = pnl + $5,
                        last_updated = NOW()
                    WHERE account_id = $1 AND token_id = $2
                    "#
                )
                .bind(&trading_account)
                .bind(&token_id)
                .bind(&eth_amount)
                .bind(should_update_duration)
                .bind(&sell_pnl)
                .execute(&mut **tx)
                .await?;
                
                // Clean up zero balance record - commenting delete part to save people's reputation for past holding
                // sqlx::query(
                //     "DELETE FROM token_balance WHERE account_id = $1 AND token_id = $2 AND holdings_value = 0"
                // )
                // .bind(&trading_account)
                // .bind(&token_id)
                // .execute(&mut **tx)
                // .await?;
                
                println!("Position closed for account: {}, token: {}", trading_account, token_id);
                
            } else {
                // Partial sell - update holdings and PnL
                
                sqlx::query(
                    r#"
                    UPDATE token_balance
                    SET
                        volume = volume + $3,
                        sell_volume = sell_volume + $3,
                        holdings_value = $4,
                        pnl = pnl + $5,
                        holding_duration = CASE 
                            WHEN $6 = true AND (token_balance.holding_duration IS NULL OR token_balance.holding_duration = 0)
                            THEN EXTRACT(EPOCH FROM (NOW() - token_balance.first_bought))
                            ELSE token_balance.holding_duration
                        END,
                        last_updated = NOW()
                    WHERE account_id = $1 AND token_id = $2
                    "#
                )
                .bind(&trading_account)
                .bind(&token_id)
                .bind(&eth_amount)
                .bind(&new_holdings)
                .bind(&sell_pnl)
                .bind(should_update_duration)
                .execute(&mut **tx)
                .await?;
            }
            
            println!("SELL transaction completed for account: {}, sold {} tokens for {} ETH", trading_account, tokens_to_sell, eth_amount);
            
        } else {
            // User trying to sell tokens they don't have - fetch latest balance from smart contract
            println!("WARNING: User {} trying to sell {} tokens they don't have for token {} - fetching latest balance from contract", trading_account, token_amount, token_id);
            
            // Fetch the latest balance from the smart contract
            let contract_balance_result = get_token_balance_from_contract(&token_id, &trading_account).await;
            match contract_balance_result {
                Result::Ok(Some(contract_balance)) => {
                    println!("Contract balance for user {} on token {}: {}", trading_account, token_id, contract_balance);
                    
                    if contract_balance == BigDecimal::from(0) {
                        // User has no tokens, skip
                        println!("User has no tokens in contract, skipping sell");
                        return Ok(());
                    } else {
                        // User has tokens, create/update record with contract balance                        
                        let sell_pnl = BigDecimal::from(0); // No cost basis available, set PnL to 0
                        
                        sqlx::query(
                            r#"
                            INSERT INTO token_balance (
                                account_id, token_id, first_bought, volume, buy_volume, sell_volume, holding_duration,
                                pnl, cost_basis, holdings_value, duration_z, pnl_z, value_z, volume_z, buy_volume_z, sell_volume_z, last_updated
                            )
                            VALUES ($1, $2, NOW(), $3, 0, $3, 4800, $4, $5, $6, 0, 0, 0, 0, 0, 0, NOW())
                            ON CONFLICT (account_id, token_id)
                            DO UPDATE SET
                                volume = token_balance.volume + $3,
                                sell_volume = token_balance.sell_volume + $3,
                                holdings_value = $6,
                                pnl = token_balance.pnl + $4,
                                cost_basis = CASE 
                                    WHEN token_balance.cost_basis = 0 THEN $5
                                    ELSE token_balance.cost_basis
                                END,
                                last_updated = NOW()
                            "#
                        )
                        .bind(&trading_account)
                        .bind(&token_id)
                        .bind(&eth_amount)
                        .bind(&sell_pnl)
                        .bind(&price_per_token) // Use current price as cost basis
                        .bind(&contract_balance)
                        .execute(&mut **tx)
                        .await?;
                        
                        println!("Updated token balance from contract for user {} on token {}: {}", trading_account, token_id, contract_balance);
                    }
                }
                Result::Ok(None) => {
                    println!("Failed to fetch contract balance for user {} on token {} - skipping sell", trading_account, token_id);
                    return Ok(());
                }
                Err(e) => {
                    println!("Error fetching contract balance for user {} on token {}: {} - skipping sell", trading_account, token_id, e);
                    return Ok(());
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Update cult_token volume stats
    // ------------------------------------------------------------------
    sqlx::query(
        r#"
        UPDATE cult_token
        SET volume = volume + $2
        WHERE id = $1
        "#
    )
    .bind(&token_id)
    .bind(&eth_amount)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

fn deserialize_u128_from_str<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where D: Deserializer<'de>
{
    let s = String::deserialize(deserializer)?;
    u128::from_str(&s).map_err(de::Error::custom)
}
fn serialize_u128_to_str<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

// Event structs
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct CultTokenCreatedEvent {
    pub token_address: String,
    pub token_creator: String,
    pub airdrop_contract: String,
    pub factory_address: String,
    pub protocol_fee_recipient: String,
    pub token_uri: String,
    pub name: String,
    pub symbol: String,
    pub pool_address: String,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub transaction_hash: String,
    pub chain_id: String,
    pub merkle_roots: Vec<String>,      
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    #[serde(serialize_with = "serialize_u128_to_str")]
    pub total_amount: u128,         
    pub total_airdrop_recipient_count: u32,
}


// Update CultTokenTransferEvent struct to include token_id
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct CultTokenTransferEvent {
    pub from: String,
    pub to: String,
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    #[serde(serialize_with = "serialize_u128_to_str")]
    pub from_token_balance: u128,
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    #[serde(serialize_with = "serialize_u128_to_str")]
    pub to_token_balance: u128,
    pub block_timestamp: u64,
    pub transaction_hash: String,
    pub token_id: String,
}

#[derive(serde::Deserialize)]
pub struct CultTokenFeesEvent {
    pub order_referrer: String,
    pub order_referrer_fee: i64,
    pub block_timestamp: u64,
    pub transaction_hash: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TokensClaimedEvent {
    pub token_id: String,
    pub recipient_id: String,
    pub amount: BigDecimal,
    pub block_timestamp: u64,
    pub transaction_hash: String,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct CultSwapEvent {
    pub sender: String,
    pub recipient: String,
    pub amount0: BigDecimal,
    pub amount1: BigDecimal,
    pub sqrt_price_x96: BigDecimal,
    pub liquidity: BigDecimal,
    pub tick: i64,
    pub pool_address: String,
    pub transaction_hash: String,
    pub block_timestamp: u64,
}