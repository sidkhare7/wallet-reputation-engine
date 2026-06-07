use alloy::primitives::Address;
use alloy_provider::ProviderBuilder;
use anyhow::Result;
use log::{error, info, warn};
use sqlx::{PgPool, Row};
use std::env;
use std::str::FromStr;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use futures::future::join_all;
use std::fs::File;
use std::io::Write;
use tokio::sync::Mutex;
use chrono;


/// Provider manager that handles multiple RPC endpoints with round-robin cycling and failover
#[derive(Debug, Clone)]
pub struct ProviderManager {
    providers: Vec<Arc<alloy_provider::fillers::FillProvider<alloy_provider::fillers::JoinFill<alloy_provider::Identity, alloy_provider::fillers::JoinFill<alloy_provider::fillers::GasFiller, alloy_provider::fillers::JoinFill<alloy_provider::fillers::BlobGasFiller, alloy_provider::fillers::JoinFill<alloy_provider::fillers::NonceFiller, alloy_provider::fillers::ChainIdFiller>>>>, alloy_provider::RootProvider>>>,
    current_index: Arc<Mutex<usize>>,
    rpc_urls: Vec<String>,
}

impl ProviderManager {
    /// Create a new provider manager with multiple RPC URLs
    pub async fn new() -> Result<Self> {
        let rpc_urls = vec![
            env::var("MONAD_TESTNET_RPC_URL").unwrap_or_default(),
            env::var("MONAD_TESTNET_RPC_URL_2").unwrap_or_default(),
            env::var("MONAD_TESTNET_RPC_URL_3").unwrap_or_default(),
            env::var("MONAD_TESTNET_RPC_URL_4").unwrap_or_default(),
        ];

        // Filter out empty URLs
        let valid_urls: Vec<String> = rpc_urls.into_iter()
            .filter(|url| !url.is_empty())
            .collect();

        if valid_urls.is_empty() {
            return Err(anyhow::anyhow!("No valid RPC URLs found in environment variables"));
        }

        info!("Initializing provider manager with {} RPC endpoints", valid_urls.len());

        // Create providers for all valid URLs
        let mut providers = Vec::new();
        for (i, url) in valid_urls.iter().enumerate() {
            match url.parse() {
                Ok(parsed_url) => {
                    let provider = Arc::new(ProviderBuilder::new().on_http(parsed_url));
                    providers.push(provider);
                    info!("Successfully created provider {} for URL: {}", i + 1, url);
                }
                Err(e) => {
                    warn!("Failed to parse RPC URL {}: {}", url, e);
                    continue;
                }
            }
        }

        if providers.is_empty() {
            return Err(anyhow::anyhow!("Failed to create any valid providers"));
        }

        Ok(Self {
            providers,
            current_index: Arc::new(Mutex::new(0)),
            rpc_urls: valid_urls,
        })
    }

    /// Get the next provider using round-robin cycling
    async fn get_next_provider(&self) -> Arc<alloy_provider::fillers::FillProvider<alloy_provider::fillers::JoinFill<alloy_provider::Identity, alloy_provider::fillers::JoinFill<alloy_provider::fillers::GasFiller, alloy_provider::fillers::JoinFill<alloy_provider::fillers::BlobGasFiller, alloy_provider::fillers::JoinFill<alloy_provider::fillers::NonceFiller, alloy_provider::fillers::ChainIdFiller>>>>, alloy_provider::RootProvider>> {
        let mut index = self.current_index.lock().await;
        let provider = self.providers[*index].clone();
        *index = (*index + 1) % self.providers.len();
        provider
    }

    /// Execute a function with failover across all providers
    pub async fn execute_with_failover<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn(Arc<alloy_provider::fillers::FillProvider<alloy_provider::fillers::JoinFill<alloy_provider::Identity, alloy_provider::fillers::JoinFill<alloy_provider::fillers::GasFiller, alloy_provider::fillers::JoinFill<alloy_provider::fillers::BlobGasFiller, alloy_provider::fillers::JoinFill<alloy_provider::fillers::NonceFiller, alloy_provider::fillers::ChainIdFiller>>>>, alloy_provider::RootProvider>>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>> + Send + Sync,
        T: Send,
    {
        let mut last_error = None;
        let mut backoff_ms = 100; // Start with 100ms backoff
        
        // Try each provider with exponential backoff on rate limits
        for attempt in 0..self.providers.len() {
            let provider = self.get_next_provider().await;
            let provider_index = {
                let index = self.current_index.lock().await;
                (*index + self.providers.len() - 1) % self.providers.len()
            };
            
            match operation(provider).await {
                Ok(result) => {
                    if attempt > 0 {
                        info!("Successfully executed operation with provider {} after {} failed attempts", 
                            provider_index + 1, attempt);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    let error_str = e.to_string();
                    let is_rate_limit = error_str.contains("429") || error_str.contains("rate limit") || error_str.contains("compute units");
                    
                    if is_rate_limit {
                        warn!("Provider {} hit rate limit (attempt {}): {}", provider_index + 1, attempt + 1, e);
                        
                        // Exponential backoff for rate limits
                        if attempt < self.providers.len() - 1 {
                            info!("Rate limited - waiting {}ms before trying next provider", backoff_ms);
                            tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                            backoff_ms = std::cmp::min(backoff_ms * 2, 5000); // Cap at 5 seconds
                        }
                    } else {
                        warn!("Provider {} failed (attempt {}): {}", provider_index + 1, attempt + 1, e);
                        
                        // Shorter delay for non-rate-limit errors
                        if attempt < self.providers.len() - 1 {
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        }
                    }
                    
                    last_error = Some(e);
                }
            }
        }

        // All providers failed
        error!("All {} providers failed", self.providers.len());
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All providers failed with unknown errors")))
    }

    /// Get the number of available providers
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Get the current RPC URLs being used
    pub fn get_rpc_urls(&self) -> &[String] {
        &self.rpc_urls
    }
}

/// Lookup the balanceOf a token using the provider manager with failover
pub async fn get_token_balance_with_manager(
    provider_manager: &ProviderManager,
    token_address: &str,
    account_address: &str,
) -> Result<Option<sqlx::types::BigDecimal>> {
    let token_address = token_address.to_string();
    let account_address = account_address.to_string();
    
    provider_manager.execute_with_failover(|provider| {
        let token_address = token_address.clone();
        let account_address = account_address.clone();
        
        Box::pin(async move {
            get_token_balance_with_provider(&*provider, &token_address, &account_address).await
        })
    }).await
}

/// Lookup the balanceOf a token given the token address and account address
/// This function queries the smart contract directly to get the current token balance
/// Uses a provided provider to avoid creating new connections
pub async fn get_token_balance_with_provider<P>(
    provider: &P,
    token_address: &str,
    account_address: &str,
) -> Result<Option<sqlx::types::BigDecimal>>
where
    P: alloy_provider::Provider + Clone,
{
    // Parse addresses with error handling
    let parse_address = |s: &str, desc: &str| match Address::from_str(s) {
        Ok(addr) => Some(addr),
        Err(_) => {
            error!("Invalid {} address format: {}", desc, s);
            None
        }
    };

    let Some(token_addr) = parse_address(token_address, "token") else {
        return Ok(None);
    };
    let Some(account_addr) = parse_address(account_address, "account") else {
        return Ok(None);
    };

    // Use the proper CultV2 ABI for balanceOf function
    let cult_v2_abi = serde_json::json!([
        {
            "inputs": [
                {
                    "internalType": "address",
                    "name": "account",
                    "type": "address"
                }
            ],
            "name": "balanceOf",
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

    // Parse JSON into JsonAbi
    let json_abi: alloy::json_abi::JsonAbi = serde_json::from_value(cult_v2_abi)?;

    // Create contract instance with the provided provider
    let contract = alloy::contract::ContractInstance::new(
        token_addr,
        provider,
        alloy::contract::Interface::new(json_abi)
    );

    // Call balanceOf function
    match contract.function("balanceOf", &[alloy::dyn_abi::DynSolValue::Address(account_addr)]) {
        Ok(call) => {
            match call.call().await {
                Ok(result) => {
                    // Decode the result
                    if let Some(alloy::dyn_abi::DynSolValue::Uint(balance, _)) = result.first() {
                        sqlx::types::BigDecimal::from_str(&balance.to_string())
                            .map(|balance_bd| {
                                info!("Successfully fetched token balance from contract: {} for account: {}", 
                                    balance_bd, account_address);
                                Some(balance_bd)
                            })
                            .map_err(|e| {
                                error!("Failed to parse balance to BigDecimal: {}", e);
                                e
                            })
                            .map(Ok)
                            .unwrap_or_else(|_| Ok(None))
                    } else {
                        error!("Unexpected result type from balanceOf call");
                        Ok(None)
                    }
                }
                Err(e) => {
                    error!("Failed to call balanceOf on token {} for account {}: {}", 
                        token_address, account_address, e);
                    Ok(None)
                }
            }
        }
        Err(e) => {
            error!("Failed to create balanceOf function call: {}", e);
            Ok(None)
        }
    }
}

/// Original function for backward compatibility - now uses provider manager with failover
pub async fn get_token_balance_from_contract(
    token_address: &str,
    account_address: &str,
) -> Result<Option<sqlx::types::BigDecimal>> {
    // Create provider manager with failover
    match ProviderManager::new().await {
        Ok(provider_manager) => {
            get_token_balance_with_manager(&provider_manager, token_address, account_address).await
        }
        Err(e) => {
            error!("Failed to create provider manager: {}", e);
            Ok(None)
        }
    }
}

#[derive(Debug, Clone)]
struct AccountBalance {
    account_id: String,
    current_holdings: sqlx::types::BigDecimal,
}

#[derive(Debug, Clone)]
struct BalanceUpdate {
    account_id: String,
    old_balance: sqlx::types::BigDecimal,
    new_balance: sqlx::types::BigDecimal,
}

#[derive(Debug, Clone, Serialize)]
struct AccountBalanceRecord {
    account_id: String,
    token_id: String,
    database_balance: String,
    contract_balance: String,
    balance_difference: String,
    timestamp: String,
}

/// Update holdings_value for ALL accounts for a specific token and export data to CSV
/// This function reads all accounts from the database (treating missing token balances as 0), 
/// fetches their current balance from the smart contract in parallel batches, 
/// updates their holdings_value in token_balance table, and exports all data to a CSV file
pub async fn update_token_holdings_from_contract(
    pool: &PgPool,
    token_id: &str,
) -> Result<UpdateResult> {
    info!("Starting parallel update of token holdings for token: {}", token_id);
    
    // Create provider manager with multiple RPC endpoints
    let provider_manager = ProviderManager::new().await?;
    info!("Created provider manager with {} RPC endpoints", provider_manager.provider_count());
    
    // Get ALL accounts and their token_balance entries for this token (treat missing as 0)
    let rows = sqlx::query(
        "SELECT a.id as account_id, COALESCE(tb.holdings_value, 0) as holdings_value 
         FROM account a 
         LEFT JOIN token_balance tb ON a.id = tb.account_id AND tb.token_id = $1"
    )
    .bind(token_id)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        info!("No accounts found in the database");
        return Ok(UpdateResult {
            token_id: token_id.to_string(),
            total_accounts: 0,
            successful_updates: 0,
            failed_updates: 0,
            errors: vec![],
        });
    }

    info!("Found {} accounts to check for token: {}", rows.len(), token_id);

    // Convert rows to AccountBalance structs
    let accounts: Vec<AccountBalance> = rows
        .into_iter()
        .map(|row| AccountBalance {
            account_id: row.get("account_id"),
            current_holdings: row.get("holdings_value"),
        })
        .collect();

    let mut result = UpdateResult {
        token_id: token_id.to_string(),
        total_accounts: accounts.len(),
        successful_updates: 0,
        failed_updates: 0,
        errors: vec![],
    };

    // Process accounts in parallel batches
    let batch_size = 50; // Reduce batch size to avoid overwhelming RPC endpoints
    let max_concurrent = 10; // Reduce concurrent calls to stay within rate limits
    let call_delay = 50; // Increase delay between calls to 50ms
    let batch_delay = 500; // Increase delay between batches to 500ms
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
    let balance_update_threshold = sqlx::types::BigDecimal::from(1_000_000_000i64);

    let mut balance_updates = Vec::new();
    let mut csv_records = Vec::new();
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    
    for (batch_index, batch) in accounts.chunks(batch_size).enumerate() {
        info!("Processing batch {} of {} accounts", batch_index + 1, batch.len());
        
        // Create futures for parallel contract calls within this batch
        let batch_futures: Vec<_> = batch
            .iter()
            .map(|account| {
                let token_id = token_id.to_string();
                let account_clone = account.clone();
                let semaphore_clone = semaphore.clone();
                let provider_manager_clone = provider_manager.clone(); // Use the provider manager
                
                async move {
                    let _permit = semaphore_clone.acquire().await.unwrap();
                    
                    // Add small delay between individual calls
                    tokio::time::sleep(std::time::Duration::from_millis(call_delay)).await;
                    
                    // Retry logic for rate-limited requests
                    let mut retry_count = 0;
                    let max_retries = 3;
                    let mut retry_delay = 200; // Start with 200ms
                    
                    loop {
                        match get_token_balance_with_manager(&provider_manager_clone, &token_id, &account_clone.account_id).await {
                            Ok(Some(new_balance)) => {
                                if retry_count > 0 {
                                    info!("Successfully fetched balance for {} after {} retries", account_clone.account_id, retry_count);
                                }
                                return Ok(BalanceUpdate {
                                    account_id: account_clone.account_id,
                                    old_balance: account_clone.current_holdings,
                                    new_balance,
                                });
                            }
                            Ok(None) => {
                                return Err(format!("Failed to fetch balance from contract for account: {}", account_clone.account_id));
                            }
                            Err(e) => {
                                let error_str = e.to_string();
                                let is_rate_limit = error_str.contains("429") || error_str.contains("rate limit") || error_str.contains("compute units");
                                
                                if is_rate_limit && retry_count < max_retries {
                                    retry_count += 1;
                                    warn!("Rate limited for account {} (retry {}/{}), waiting {}ms", 
                                        account_clone.account_id, retry_count, max_retries, retry_delay);
                                    tokio::time::sleep(std::time::Duration::from_millis(retry_delay)).await;
                                    retry_delay *= 2; // Exponential backoff
                                    continue; // Retry the request
                                } else {
                                    return Err(format!("Error fetching balance for account {} after {} retries: {}", 
                                        account_clone.account_id, retry_count, e));
                                }
                            }
                        }
                    }
                }
            })
            .collect();
        
        // Wait for all futures in this batch to complete
        let batch_results = join_all(batch_futures).await;
        
        // Process results from this batch
        for batch_result in batch_results {
            match batch_result {
                Ok(balance_update) => {
                    // Calculate the difference between new and old balance
                    let difference = &balance_update.new_balance - &balance_update.old_balance;
                    let csv_record = AccountBalanceRecord {
                        account_id: balance_update.account_id.clone(),
                        token_id: token_id.to_string(),
                        database_balance: balance_update.old_balance.to_string(),
                        contract_balance: balance_update.new_balance.to_string(),
                        balance_difference: difference.to_string(),
                        timestamp: timestamp.clone(),
                    };
                    csv_records.push(csv_record);
                    
                    // Only update database if absolute difference is greater than 1,000,000,000
                    
                    let abs_difference = difference.abs();
                    
                    if abs_difference > balance_update_threshold {
                        info!("Queuing update for account {} - difference: {}", 
                            balance_update.account_id, difference);
                        balance_updates.push(balance_update);
                    } else {
                        info!("Skipping update for account {} - difference {} is below threshold {}", 
                            balance_update.account_id, difference, balance_update_threshold);
                    }
                    
                    result.successful_updates += 1;
                }
                Err(error_msg) => {
                    result.failed_updates += 1;
                    error!("{}", error_msg);
                    result.errors.push(error_msg);
                }
            }
        }
        
        info!("Completed batch {}. Total successful: {}, failed: {}", 
            batch_index + 1, result.successful_updates, result.failed_updates);
        
        // Add delay between batches to prevent API overload
        if batch_index < accounts.chunks(batch_size).len() - 1 {
            info!("Waiting 2 seconds before next batch to prevent API overload...");
            tokio::time::sleep(std::time::Duration::from_millis(batch_delay)).await;
        }
    }

    // Batch update the database in smaller transactions to prevent timeouts
    if !balance_updates.is_empty() {
        info!("Updating {} account balances in database using bulk upsert", balance_updates.len());
        
        // Use a single transaction with bulk UPSERT for maximum performance
        let mut tx = pool.begin().await?;
        
        // Build bulk upsert query - much faster than individual updates
        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO token_balance (account_id, token_id, holdings_value, last_updated) VALUES "
        );
        
        let mut first = true;
        for update in &balance_updates {
            if !first {
                query_builder.push(", ");
            }
            first = false;
            
            query_builder.push("(")
                .push_bind(&update.account_id)
                .push(", ")
                .push_bind(token_id)
                .push(", ")
                .push_bind(&update.new_balance)
                .push(", NOW())");
        }
        
        query_builder.push(
            " ON CONFLICT (account_id, token_id) 
             DO UPDATE SET 
                holdings_value = EXCLUDED.holdings_value,
                last_updated = EXCLUDED.last_updated"
        );
        
        match query_builder.build().execute(&mut *tx).await {
            Ok(db_result) => {
                let affected_rows = db_result.rows_affected();
                info!("Bulk upserted {} account balances in single transaction", affected_rows);
                
                match tx.commit().await {
                    Ok(_) => {
                        info!("Successfully committed bulk database update");
                    }
                    Err(e) => {
                        error!("Failed to commit bulk transaction: {}", e);
                        result.failed_updates += balance_updates.len();
                        result.successful_updates = result.successful_updates.saturating_sub(balance_updates.len());
                        result.errors.push(format!("Failed to commit bulk transaction: {}", e));
                    }
                }
            }
            Err(e) => {
                error!("Failed to execute bulk upsert: {}", e);
                tx.rollback().await?;
                
                // Fallback to smaller batches if bulk insert fails
                info!("Falling back to smaller batch updates due to bulk insert failure");
                
                let fallback_batch_size = 1000; // Still much larger than before
                for (i, batch) in balance_updates.chunks(fallback_batch_size).enumerate() {
                    let mut fallback_tx = pool.begin().await?;
                    let mut batch_failed = false;
                    
                    // Use batch insert for fallback too
                    let mut fallback_builder = sqlx::QueryBuilder::new(
                        "INSERT INTO token_balance (account_id, token_id, holdings_value, last_updated) VALUES "
                    );
                    
                    let mut first_fallback = true;
                    for update in batch {
                        if !first_fallback {
                            fallback_builder.push(", ");
                        }
                        first_fallback = false;
                        
                        fallback_builder.push("(")
                            .push_bind(&update.account_id)
                            .push(", ")
                            .push_bind(token_id)
                            .push(", ")
                            .push_bind(&update.new_balance)
                            .push(", NOW())");
                    }
                    
                    fallback_builder.push(
                        " ON CONFLICT (account_id, token_id) 
                         DO UPDATE SET 
                            holdings_value = EXCLUDED.holdings_value,
                            last_updated = EXCLUDED.last_updated"
                    );
                    
                    match fallback_builder.build().execute(&mut *fallback_tx).await {
                        Ok(_) => {
                            fallback_tx.commit().await?;
                            info!("Committed fallback batch {} with {} updates", i + 1, batch.len());
                        }
                        Err(e) => {
                            result.failed_updates += batch.len();
                            result.successful_updates = result.successful_updates.saturating_sub(batch.len());
                            let error_msg = format!("Failed fallback batch {}: {}", i + 1, e);
                            error!("{}", error_msg);
                            result.errors.push(error_msg);
                            fallback_tx.rollback().await?;
                            batch_failed = true;
                        }
                    }
                    
                    if batch_failed {
                        break; // Stop processing if fallback also fails
                    }
                }
            }
        }
    }

    // Save CSV data
    if !csv_records.is_empty() {
        let csv_filename = format!("token_balance_export_{}_{}.csv", 
            token_id.replace("0x", ""), 
            chrono::Utc::now().format("%Y%m%d_%H%M%S"));
        
        match save_csv_data(&csv_records, &csv_filename) {
            Ok(_) => {
                info!("Successfully saved {} records to CSV file: {}", csv_records.len(), csv_filename);
            }
            Err(e) => {
                error!("Failed to save CSV file {}: {}", csv_filename, e);
                result.errors.push(format!("Failed to save CSV: {}", e));
            }
        }
    }

    info!("Parallel update completed for token {}: {} successful, {} failed out of {} total accounts", 
        token_id, result.successful_updates, result.failed_updates, result.total_accounts);

    Ok(result)
}

/// Save account balance data to CSV file
fn save_csv_data(records: &[AccountBalanceRecord], filename: &str) -> Result<()> {
    let mut file = File::create(filename)?;
    
    // Write CSV header
    writeln!(file, "account_id,token_id,database_balance,contract_balance,balance_difference,timestamp")?;
    
    // Write data rows
    for record in records {
        writeln!(file, "{},{},{},{},{},{}", 
            record.account_id,
            record.token_id,
            record.database_balance,
            record.contract_balance,
            record.balance_difference,
            record.timestamp
        )?;
    }
    
    Ok(())
}

/// Update holdings_value for all tokens by fetching balances from smart contracts
/// This function processes all unique tokens in the token_balance table
pub async fn update_all_token_holdings_from_contracts(
    pool: &PgPool,
) -> Result<Vec<UpdateResult>> {
    info!("Starting update of all token holdings from smart contracts");

    // // Get all unique tokens that have token_balance entries
    // let rows = sqlx::query("SELECT DISTINCT token_id FROM token_balance ORDER BY token_id")
    //     .fetch_all(pool)
    //     .await?;

    let token_ids = vec!["0x69c29fe56771f295a73Af25cc70edf58aF5954d5".to_string()];

    if token_ids.is_empty() {
        info!("No tokens found in token_balance table");
        return Ok(vec![]);
    }

    info!("Found {} unique tokens to process", token_ids.len());

    let mut results = Vec::new();

    // Process each token
    for token_id in token_ids {
        match update_token_holdings_from_contract(pool, &token_id).await {
            Ok(result) => {
                results.push(result);
            }
            Err(e) => {
                error!("Failed to update token holdings for token {}: {}", token_id, e);
                results.push(UpdateResult {
                    token_id,
                    total_accounts: 0,
                    successful_updates: 0,
                    failed_updates: 0,
                    errors: vec![format!("Failed to process token: {}", e)],
                });
            }
        }
    }

    info!("Completed update of all token holdings. Processed {} tokens", results.len());
    Ok(results)
}

/// Result structure for token balance update operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResult {
    pub token_id: String,
    pub total_accounts: usize,
    pub successful_updates: usize,
    pub failed_updates: usize,
    pub errors: Vec<String>,
}

impl UpdateResult {
    /// Check if the update was completely successful
    pub fn is_success(&self) -> bool {
        self.failed_updates == 0
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_accounts == 0 {
            100.0
        } else {
            (self.successful_updates as f64 / self.total_accounts as f64) * 100.0
        }
    }
}

/// Clean up duplicate accounts by deleting non-checksum versions when checksum versions exist
/// This function finds accounts with case-insensitive duplicates, identifies which are checksum vs non-checksum,
/// and deletes the non-checksum accounts if the checksum version has token balances
pub async fn cleanup_duplicate_accounts(pool: &PgPool) -> Result<DuplicateCleanupResult> {
    info!("Starting cleanup of duplicate accounts");
    
    // Query to find all accounts that have case-insensitive duplicates
    let duplicate_query = r#"
        SELECT DISTINCT LOWER(id) as lower_id, 
               array_agg(id ORDER BY id) as all_versions
        FROM account
        WHERE LOWER(id) IN (
            SELECT LOWER(id)
            FROM account
            GROUP BY LOWER(id)
            HAVING COUNT(*) > 1
        )
        GROUP BY LOWER(id)
        ORDER BY lower_id
    "#;
    
    let duplicate_rows = sqlx::query(duplicate_query).fetch_all(pool).await?;
    
    if duplicate_rows.is_empty() {
        info!("No duplicate accounts found");
        return Ok(DuplicateCleanupResult {
            duplicate_groups: 0,
            accounts_deleted: 0,
            accounts_preserved: 0,
            errors: vec![],
        });
    }
    
    info!("Found {} groups of duplicate accounts", duplicate_rows.len());
    
    let mut result = DuplicateCleanupResult {
        duplicate_groups: duplicate_rows.len(),
        accounts_deleted: 0,
        accounts_preserved: 0,
        errors: vec![],
    };
    
    // Process each group of duplicates
    for row in duplicate_rows {
        let lower_id: String = row.get("lower_id");
        let all_versions: Vec<String> = row.get("all_versions");
        
        info!("Processing duplicate group for '{}' with {} versions: {:?}", 
            lower_id, all_versions.len(), all_versions);
        
        // Find the checksum version
        let mut checksum_version: Option<String> = None;
        let mut non_checksum_versions: Vec<String> = Vec::new();
        
        for version in &all_versions {
            match crate::utils::misc::to_checksum_address(version) {
                Ok(checksum_addr) => {
                    if checksum_addr == *version {
                        // This is already in checksum format
                        if checksum_version.is_none() {
                            checksum_version = Some(version.clone());
                        } else {
                            // Multiple checksum versions - this shouldn't happen but let's handle it
                            let error_msg = format!("Multiple checksum versions found for {}: {} and {}", 
                                lower_id, checksum_version.as_ref().unwrap(), version);
                            warn!("{}", error_msg);
                            result.errors.push(error_msg);
                        }
                    } else {
                        // This is not in checksum format
                        non_checksum_versions.push(version.clone());
                    }
                }
                Err(e) => {
                    let error_msg = format!("Invalid address format '{}': {}", version, e);
                    error!("{}", error_msg);
                    result.errors.push(error_msg);
                }
            }
        }
        
        // If we have a checksum version, delete all non-checksum versions
        if let Some(checksum_addr) = &checksum_version {
            // Check if checksum version has any token balances for logging purposes
            let balance_check = sqlx::query!(
                "SELECT COUNT(*) as balance_count FROM token_balance WHERE account_id = $1",
                checksum_addr
            ).fetch_one(pool).await;
            
            let balance_count = match balance_check {
                Ok(balance_row) => balance_row.balance_count.unwrap_or(0),
                Err(_) => 0
            };
            
            info!("Checksum version '{}' has {} token balances, deleting {} non-checksum versions", 
                checksum_addr, balance_count, non_checksum_versions.len());
            
            // Delete all non-checksum versions and their data
            for non_checksum_addr in &non_checksum_versions {
                match delete_account_with_balances(pool, non_checksum_addr).await {
                    Ok(deleted) => {
                        if deleted {
                            result.accounts_deleted += 1;
                            info!("Successfully deleted non-checksum account and all data: {}", non_checksum_addr);
                        } else {
                            let error_msg = format!("Account {} was not found for deletion", non_checksum_addr);
                            warn!("{}", error_msg);
                            result.errors.push(error_msg);
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to delete account {}: {}", non_checksum_addr, e);
                        error!("{}", error_msg);
                        result.errors.push(error_msg);
                    }
                }
            }
            
            result.accounts_preserved += 1; // The checksum version is preserved
        } else {
            // No checksum version found, preserve all
            info!("No checksum version found for '{}', preserving all {} versions", 
                lower_id, all_versions.len());
            result.accounts_preserved += all_versions.len();
        }
    }
    
    info!("Duplicate account cleanup completed. Groups: {}, Deleted: {}, Preserved: {}, Errors: {}", 
        result.duplicate_groups, result.accounts_deleted, result.accounts_preserved, result.errors.len());
    
    Ok(result)
}

/// Delete an account and all its associated data (token balances, etc.)
/// This is used when we know we have a checksum version with the same data
/// Handles all foreign key dependencies in the correct order
async fn delete_account_with_balances(pool: &PgPool, account_id: &str) -> Result<bool> {
    let mut tx = pool.begin().await?;
    
    info!("Starting deletion of account {} and all associated data", account_id);
    
    // 1. Delete from airdrop_recipients (references account_id)
    let airdrop_recipients_delete = sqlx::query!(
        "DELETE FROM airdrop_recipients WHERE account_id = $1",
        account_id
    ).execute(&mut *tx).await?;
    info!("Deleted {} airdrop recipient records for account {}", 
        airdrop_recipients_delete.rows_affected(), account_id);
    
    // 2. Delete from token_balance (references account_id)
    let token_balance_delete = sqlx::query!(
        "DELETE FROM token_balance WHERE account_id = $1",
        account_id
    ).execute(&mut *tx).await?;
    info!("Deleted {} token balance records for account {}", 
        token_balance_delete.rows_affected(), account_id);
    
    // 3. Delete from account_communities (references account_id)
    let account_communities_delete = sqlx::query!(
        "DELETE FROM account_communities WHERE account_id = $1",
        account_id
    ).execute(&mut *tx).await?;
    info!("Deleted {} account community records for account {}", 
        account_communities_delete.rows_affected(), account_id);
    
    // 4. Delete from diamond_hand_list (references account_id)
    let diamond_hands_delete = sqlx::query!(
        "DELETE FROM diamond_hand_list WHERE account_id = $1",
        account_id
    ).execute(&mut *tx).await?;
    info!("Deleted {} diamond hands records for account {}", 
        diamond_hands_delete.rows_affected(), account_id);
    
    // 5. Update any accounts that reference this account as referrer
    // The schema has ON DELETE SET NULL, but let's be explicit
    let referrer_update = sqlx::query!(
        "UPDATE account SET referrer_id = NULL WHERE referrer_id = $1",
        account_id
    ).execute(&mut *tx).await?;
    if referrer_update.rows_affected() > 0 {
        info!("Updated {} accounts that had {} as referrer", 
            referrer_update.rows_affected(), account_id);
    }
    
    // 6. Finally, delete the account itself
    let account_delete = sqlx::query!(
        "DELETE FROM account WHERE id = $1",
        account_id
    ).execute(&mut *tx).await?;
    
    if account_delete.rows_affected() > 0 {
        tx.commit().await?;
        info!("Successfully deleted account {} and all associated data", account_id);
        
        // Log summary of what was deleted
        info!("Deletion summary for {}: {} airdrop recipients, {} token balances, {} communities, {} diamond hands, {} referrer updates", 
            account_id,
            airdrop_recipients_delete.rows_affected(),
            token_balance_delete.rows_affected(), 
            account_communities_delete.rows_affected(),
            diamond_hands_delete.rows_affected(),
            referrer_update.rows_affected()
        );
        
        Ok(true)
    } else {
        tx.rollback().await?;
        warn!("Account {} was not found for deletion", account_id);
        Ok(false)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateCleanupResult {
    pub duplicate_groups: usize,
    pub accounts_deleted: usize,
    pub accounts_preserved: usize,
    pub errors: Vec<String>,
}