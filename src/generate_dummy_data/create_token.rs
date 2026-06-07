use rand::{rng, Rng};
use rand::prelude::SliceRandom;
use rand::distr::Alphanumeric;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::fs::File;
use std::io::Write;
use anyhow::{Result, anyhow};
use hex;
use std::time::{SystemTime, UNIX_EPOCH};
use reqwest::Client;
use cult_backend::handlers; 
use dotenv::dotenv;
use std::env;
// Constants for token creation
const FACTORY_ADDRESS: &str = "0x72c06ffd9015acd5314c4f550dca15a2055be9c4";
const PROTOCOL_FEE_RECIPIENT: &str = "0x60187bc4949ee2f01b507a9f77ad615093f44260";
const BONDING_CURVE: &str = "0x3e653af26547a0d5a7ca11303b143f26119505a3";
const CHAIN_ID: &str = "10143";

// Token data structure that matches the required JSON output
#[derive(Serialize, Deserialize)]
struct TokenData {
    id: String,
    event_type: String,
    data: handlers::CultTokenCreatedEvent,
    timestamp: u64,
}


// Community data from database
#[derive(Clone)]
struct Community {
    id: String,
    merkle_root: Vec<u8>,
    holder_count: i64,
}

// Helper functions
fn random_hex(bytes: usize) -> String {
    let mut rng = rng();
    let mut buffer = vec![0u8; bytes];
    rng.fill(&mut buffer[..]);
    format!("0x{}", hex::encode(buffer))
}


fn random_ipfs() -> String {

//     let ipfs = ["QmaZGkesA6wN9N6mXLcp6wJ2a3bR5bryw9f4cERWPW2cK1",
// "QmcyU4KWgs28bBCykUzFgGqkwH59vb7dzqCDjgC7E3Zs32",
// "QmeB46J21yz2CuDame2LH14BSxQrJctsuvatExtKTmy662",
// "QmQqa9sxeoEHvqMyEMqSPsMVXXBVi2zbxqkcYSenQqCC9g",
// "QmQvbmJbXg5sSYVxxixrjfK5xbmaJuif7EP3WCWrfyJxvi",
// "QmRMv5oYjvqmAKx9j9GPaPcZXfCBduTmiy8upTtcGjMUw3",
// "QmUhTVcs3Q2hB4jRfFPhduMh7aHQpfMTJsJvVJhx5p8H5X",
// "QmUtpA5YDj326t5G6XAbeRvBGrJgffZg71Prdi51Rqr3o8",
// "QmUXWnYEDZyYFeWyDpyMgXqeq5PDwyC3ikwBzRXwNrs7XH",
// "QmUYNp5Y8KdD1cbM8GKRQV7iXbhHTekCnCijdEygbgFnq9",
// "QmWoY5xUXeVDPRYZBAZXdc2wD6d92nb9oCqbHiraob6vgU",
// "QmXUwZTrikE3LjCYiWRnqXAapEk4X1Hmbg4auviHTumbNL"];
//     // let random_string: String = rng()
//     //     .sample_iter(&Alphanumeric)
//     //     .take(45)
//     //     .map(char::from)
//     //     .collect();

//     let mut rng = rng();    
//     format!("ipfs://{}", ipfs[rng.random_range(0..ipfs.len())])


    let ipfs = "ipfs://QmbZWD3m7RSAjeuVaqJHNxX6bRCELC3HfdoYtermctWVSc";
    ipfs.to_string()
}



fn random_token_name() -> String {
    let prefixes = ["Super", "Mega", "Ultra", "Hyper", "Quantum", "Cosmic", "Atomic", "Stellar", "Lunar", "Solar", "Techno", "Cyber", "Meta", "Crypto", "Digi"];
    let suffixes = ["Coin", "Token", "Chain", "Net", "Base", "Hub", "Link", "Swap", "Cash", "Pay", "Finance", "Dao", "Verse", "World", "System"];
    
    let mut rng = rng();
    format!("{}{}", 
        prefixes[rng.random_range(0..prefixes.len())], 
        suffixes[rng.random_range(0..suffixes.len())]
    ).to_lowercase()
}


fn random_token_symbol() -> String {
    let mut rng = rng();
    let length = rng.random_range(3..6);
    
    let symbol: String = rng
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();
        
    symbol.to_uppercase()
}

// Get random elements from a vector
fn random_elements<T: Clone>(vec: &[T], count: usize) -> Vec<T> {
    let mut rng = rng();
    let mut indices: Vec<usize> = (0..vec.len()).collect();
    indices.shuffle(&mut rng);
    
    let take_count = std::cmp::min(count, vec.len());
    indices.iter()
        .take(take_count)
        .map(|&idx| vec[idx].clone())
        .collect()
}

// Function to get all accounts from database
async fn get_accounts(pool: &PgPool) -> Result<Vec<String>> {
    let accounts: Vec<String> = sqlx::query("SELECT id FROM account LIMIT 500")
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| row.get::<String, _>("id"))
        .collect();
    
    if accounts.is_empty() {
        return Err(anyhow!("No accounts found in the database. Please create accounts first."));
    }
    
    Ok(accounts)
}

// Function to get all communities with their merkle roots
async fn get_communities(pool: &PgPool) -> Result<Vec<Community>> {
    let rows = sqlx::query("SELECT id, merkle_root, holder_count FROM communities")
        .fetch_all(pool)
        .await?;
        
    let communities = rows.into_iter()
        .map(|row| Community {
            id: row.get("id"),
            merkle_root: row.get("merkle_root"),
            holder_count: row.get("holder_count"),
        })
        .collect::<Vec<_>>();
    
    if communities.is_empty() {
        return Err(anyhow!("No communities found in the database. Please create communities first."));
    }
    
    Ok(communities)
}

// Create sample accounts if they don't exist
async fn create_sample_accounts(pool: &PgPool) -> Result<()> {
    // Check if account table has data
    let account_count: i64 = sqlx::query("SELECT COUNT(*) FROM account")
        .fetch_one(pool)
        .await?
        .get(0);
    
    if account_count > 0 {
        println!("Accounts already exist, skipping creation");
        return Ok(());
    }
    
    println!("Creating sample accounts...");
    
    let mut tx = pool.begin().await?;
    
    // Create sample accounts
    for i in 0..50 {
        let account_id = random_hex(20);
        let slug = format!("user-{}", i+1);
        
        sqlx::query(
            r#"
            INSERT INTO account 
            (id, slug, diamond_hand_probability, total_referrals, fee_collected)
            VALUES ($1, $2, $3, $4, $5)
            "#
        )
        .bind(&account_id)
        .bind(&slug)
        .bind(rng().random_range(0..100))
        .bind(rng().random_range(0..20))
        .bind(rng().random_range::<f64, _>(0.0..10.0))
        .execute(&mut *tx)
        .await?;
    }
    
    tx.commit().await?;
    println!("Sample accounts created");
    
    Ok(())
}

// Create sample communities if they don't exist
async fn create_sample_communities(pool: &PgPool) -> Result<()> {
    // Check if communities table exists and has data
    let community_count: i64 = sqlx::query("SELECT COUNT(*) FROM communities")
        .fetch_one(pool)
        .await?
        .get(0);
    
    if community_count > 0 {
        println!("Communities already exist, skipping creation");
        return Ok(());
    }
    
    println!("Creating sample communities...");
    
    let mut tx = pool.begin().await?;
    
    // Create sample communities
    for i in 0..5 {
        let community_id = format!("community-{}", i+1);
        let name = format!("Sample Community {}", i+1);
        let img_url = format!("https://example.com/communities/{}.jpg", i+1);
        let address = random_hex(20);
        
        // Generate random merkle root
        let mut merkle_root = vec![0u8; 32];
        rng().fill(&mut merkle_root[..]);
        
        let holder_count = rng().random_range(100..1000); // 100-999 holders
        
        // Create a sample proof for one address
        let proof_address = random_hex(20);
        let proof_obj = json!({
            proof_address: {
                "index": 0,
                "amount": "1000000",
                "proof": [random_hex(32)]
            }
        });
        
        sqlx::query(
            r#"
            INSERT INTO communities 
            (id, name, img_url, address, chain, merkle_root, last_updated_time, merkle_proofs, holder_count, community_score)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(&community_id)
        .bind(&name)
        .bind(&img_url)
        .bind(&address)
        .bind(CHAIN_ID)
        .bind(&merkle_root)
        .bind(chrono::Utc::now())
        .bind(proof_obj)
        .bind(holder_count)
        .bind(rng().random_range::<f64, _>(1.0..5.0))
        .execute(&mut *tx)
        .await?;
    }
    
    tx.commit().await?;
    println!("Sample communities created");
    
    Ok(())
}

// Main function to generate token JSON data
async fn generate_token_json(pool: &PgPool, count: usize) -> Result<Vec<TokenData>> {
    println!("Generating {} token JSON objects...", count);
    
    // Get existing accounts and communities
    let accounts = get_accounts(pool).await?;
    let communities = get_communities(pool).await?;
    
    println!("Found {} accounts and {} communities", accounts.len(), communities.len());
    
    // Generate token data
    let mut tokens = Vec::with_capacity(count);
    let current_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let one_year_in_seconds = 365 * 24 * 60 * 60;
    let start_timestamp = current_timestamp - one_year_in_seconds;
    
    for _ in 0..count {
        let mut rng = rng();
        
        // Select random creator from existing accounts
        let token_creator = accounts[rng.random_range(0..accounts.len())].clone();
        
        // Generate random timestamps and block number
        let timestamp = start_timestamp + rng.random_range(0..one_year_in_seconds);
        let block_number = 7_000_000 + rng.random_range(0..1_000_000);
        
        // Generate transaction hash and addresses
        let transaction_hash = random_hex(32);
        let token_address = random_hex(20);
        let pool_address = random_hex(20);
        let airdrop_contract = random_hex(20);
        
        // Generate token name and symbol
        let name = random_token_name();
        let symbol = random_token_symbol();
        
        // Generate token URI
        let token_uri = random_ipfs();
        
        // Select random communities (1-3 communities per token)
        let community_count = rng.random_range(1..=std::cmp::min(2, communities.len()));
        let selected_communities: Vec<Community> = random_elements(&communities, community_count);
        
        // Calculate total holder count and collect merkle roots
        let merkle_roots = selected_communities.iter()
            .map(|community| hex::encode(&community.merkle_root))
            .collect::<Vec<_>>();
        
        let total_holder_count = selected_communities.iter()
            .map(|community| community.holder_count)
            .sum::<i64>() as u64;
        
        // Generate total amount (between 100M and 1B)
        //let total_amount= (rng.random_range(1..1000) * 100_000_000).to_string();
        let total_amount: u128 = (rng.random_range(1..1000) as u128) * 100_000_000_u128;
        // Create the token JSON
        let token_data = TokenData {
            id: transaction_hash.clone(),
            event_type: "CultTokenCreated".to_string(),
            data: handlers::CultTokenCreatedEvent {
                block_number,
                block_timestamp: timestamp,
                transaction_hash,
                factory_address: FACTORY_ADDRESS.to_string(),
                token_creator,
                protocol_fee_recipient: PROTOCOL_FEE_RECIPIENT.to_string(),
                bonding_curve: BONDING_CURVE.to_string(),
                token_uri,
                name,
                symbol,
                token_address,
                pool_address,
                airdrop_contract,
                chain_id: CHAIN_ID.to_string(),
                merkle_roots,
                total_amount: total_amount,
                total_airdrop_recipient_count: total_holder_count as u32,
            },
            timestamp,
        };
        
        tokens.push(token_data);
    }
    
    println!("Successfully generated {} token JSON objects", count);
    
    Ok(tokens)
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();
    // Database connection settings
    // Configuration from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let server_address =
        env::var("SERVER_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let webhook_secret = env::var("WEBHOOK_SECRET").expect("WEBHOOK_SECRET must be set");

    println!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url).await?;
    
    // Create sample data if needed
    //create_sample_accounts(&pool).await?;
    //create_sample_communities(&pool).await?;
    
    // Generate token JSON data
    let tokens = generate_token_json(&pool, 10).await?;

    // Write to JSON file
    let json_data = serde_json::to_string_pretty(&tokens)?;
    let path = std::env::current_dir()?.join("token_creation_data.json");
    let mut file = File::create(&path)?;
    file.write_all(json_data.as_bytes())?;
    
    println!("File saved at: {}", path.display());
    
    // Send to API
    let client = Client::new();
    for token in tokens {
        let res = client
            .post("http://localhost:8080/webhook") // Replace with actual endpoint
            .header("X-Webhook-Signature", &webhook_secret)
            .header("Content-Type", "application/json")
            .json(&token)
            .send()
            .await?;

        println!("Sent token {}, status: {}", token.id, res.status());
    }

    Ok(())
}