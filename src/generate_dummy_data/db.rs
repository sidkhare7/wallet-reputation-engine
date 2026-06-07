use alloy_primitives::{Address, U256};
use alloy_signer::{k256::ecdsa::SigningKey, Signer};
use anyhow::{Result, anyhow};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::{str::FromStr, collections::HashMap};
use serde::{Serialize, Deserialize};
use bigdecimal::BigDecimal;
use std::sync::Arc;

// Enhanced account struct that includes additional data from the database
#[derive(Debug, Clone)]
pub struct EnhancedAccount {
    pub wallet: LocalWallet,
    pub account_id: String,
    pub referrer_id: Option<String>,
    pub diamond_hand_probability: i32,
    pub total_referrals: i32,
    pub fee_collected: BigDecimal,
}

// Token balance struct
#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub account_id: String,
    pub token_id: String,
    pub volume: BigDecimal,
    pub holdings_value: BigDecimal,
}

// Database connection and account loading functions
pub struct DbClient {
    pool: PgPool,
}

impl DbClient {
    // Initialize the database connection
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        
        Ok(Self { pool })
    }
    
    // Load accounts from a private keys file and fetch their data from the database
    pub async fn load_accounts_from_file_and_db(&self, file_path: &str, count: usize) -> Result<Vec<EnhancedAccount>> {
        // Read private keys from file
        let accounts_data = std::fs::read_to_string(file_path)?;
        let private_keys: Vec<String> = serde_json::from_str(&accounts_data)?;
        
        // Create wallets and get addresses
        let mut wallets = Vec::with_capacity(std::cmp::min(private_keys.len(), count));
        let mut addresses = Vec::with_capacity(std::cmp::min(private_keys.len(), count));
        
        for (i, private_key) in private_keys.iter().enumerate() {
            if i >= count {
                break;
            }
            
            // Parse private key and create wallet
            let signing_key = SigningKey::from_bytes(&hex::decode(private_key.trim_start_matches("0x"))?)?;
            let wallet = LocalWallet::from(signing_key);
            
            // Get address
            let address = wallet.address().to_string();
            
            wallets.push(wallet);
            addresses.push(address);
        }
        
        // Construct SQL placeholders for the IN clause
        let placeholders: Vec<String> = (1..=addresses.len()).map(|i| format!("${}", i)).collect();
        let placeholder_str = placeholders.join(",");
        
        // Query the database for account information
        let query = format!(
            "SELECT id, referrer_id, diamond_hand_probability, total_referrals, fee_collected 
             FROM account 
             WHERE id IN ({})", 
            placeholder_str
        );
        
        // Convert addresses to a vector of sqlx::types::Any for binding
        let address_params: Vec<&str> = addresses.iter().map(|a| a.as_str()).collect();
        
        let rows = sqlx::query(&query)
            .bind_all(address_params)
            .fetch_all(&self.pool)
            .await?;
        
        // Map database results to wallets
        let mut enhanced_accounts = Vec::with_capacity(rows.len());
        let mut address_to_wallet = HashMap::new();
        
        for (i, wallet) in wallets.into_iter().enumerate() {
            address_to_wallet.insert(addresses[i].clone(), wallet);
        }
        
        for row in rows {
            let account_id: String = row.try_get("id")?;
            let wallet = address_to_wallet.remove(&account_id)
                .ok_or_else(|| anyhow!("Wallet not found for address: {}", account_id))?;
            
            let enhanced_account = EnhancedAccount {
                wallet,
                account_id: account_id.clone(),
                referrer_id: row.try_get("referrer_id")?,
                diamond_hand_probability: row.try_get("diamond_hand_probability")?,
                total_referrals: row.try_get("total_referrals")?,
                fee_collected: row.try_get("fee_collected")?,
            };
            
            enhanced_accounts.push(enhanced_account);
        }
        
        // Log results
        println!("Loaded {} accounts from database", enhanced_accounts.len());
        
        Ok(enhanced_accounts)
    }
    
    // Get token balances for a specific cult token and accounts
    pub async fn get_token_balances(&self, token_id: &str, account_ids: &[String]) -> Result<HashMap<String, TokenBalance>> {
        // Construct SQL placeholders for the IN clause
        let placeholders: Vec<String> = (2..=(account_ids.len() + 1)).map(|i| format!("${}", i)).collect();
        let placeholder_str = placeholders.join(",");
        
        let query = format!(
            "SELECT account_id, token_id, volume, holdings_value
             FROM token_balance
             WHERE token_id = $1 AND account_id IN ({})",
            placeholder_str
        );
        
        // Create query parameters
        let mut query_builder = sqlx::query(&query)
            .bind(token_id);
            
        // Bind account IDs
        for account_id in account_ids {
            query_builder = query_builder.bind(account_id);
        }
        
        // Execute query
        let rows = query_builder
            .fetch_all(&self.pool)
            .await?;
        
        // Map results to token balances
        let mut balances = HashMap::new();
        
        for row in rows {
            let account_id: String = row.try_get("account_id")?;
            let token_balance = TokenBalance {
                account_id: account_id.clone(),
                token_id: row.try_get("token_id")?,
                volume: row.try_get("volume")?,
                holdings_value: row.try_get("holdings_value")?,
            };
            
            balances.insert(account_id, token_balance);
        }
        
        Ok(balances)
    }
}