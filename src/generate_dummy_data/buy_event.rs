use anyhow::{Result, Context};
use dotenv::dotenv;
use rand::{Rng, thread_rng};
use reqwest::Client;
use sqlx::{postgres::PgPoolOptions, PgPool, types::BigDecimal};
use serde_json::{Value, json};
use std::{
    env,
    fs::File,
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::time::{sleep, Duration};

/// Generate buy event data by fetching tokens and accounts from the database
/// Generate buy event data by fetching tokens and accounts from the database
async fn generate_buy_event_json(pool: &PgPool, count: usize) -> Result<Vec<Value>> {
    // 1. fetch tokens: id, price (wei), circulating_supply (units)
    let tokens = sqlx::query!(
        r#"
        SELECT id, price, circulating_supply 
        FROM cult_token
        "#
    )
    .fetch_all(pool)
    .await?;
    anyhow::ensure!(!tokens.is_empty(), "No tokens found");

    // 2. fetch accounts + their optional referrer
    let accounts = sqlx::query!(
        r#"
        SELECT id, referrer_id 
        FROM account
        "#
    )
    .fetch_all(pool)
    .await?;
    anyhow::ensure!(!accounts.is_empty(), "No accounts found");

    // constants for 18-decimal math and ETH ranges
    const DECIMALS: u128 = 1_000_000_000_000_000_000;   // 10^18
    const MIN_ETH: u128  = 10 * DECIMALS;               // 10 ETH in wei
    const MAX_ETH: u128  = 5000 * DECIMALS;              // 5000 ETH in wei

    let mut rng = thread_rng();
    let mut events = Vec::with_capacity(count);
    let base_ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    for i in 0..count {
        let t = &tokens[rng.gen_range(0..tokens.len())];
        let a = &accounts[rng.gen_range(0..accounts.len())];
        let order_referrer = a.referrer_id.clone().unwrap_or_else(|| a.id.clone());

        // BigDecimal → u128
        let price_wei: u128 = t.price
            .to_string()
            .parse()
            .context("parsing token.price to u128")?;
        let orig_supply: u128 = t.circulating_supply
            .to_string()
            .parse()
            .context("parsing token.circulating_supply to u128")?;

        // 1–50 ETH in wei
        let mut eth_sold = rng.gen_range(MIN_ETH..=MAX_ETH);

        // compute tokens in "smallest units": tokens_bought = eth_sold * 10^18 / price_wei
        let mut tokens_bought = if price_wei > 0 {
            eth_sold
                .checked_mul(DECIMALS)
                .unwrap()
                / price_wei
        } else {
            0
        };

        // enforce at least one whole token (10^18 units)
        if tokens_bought < DECIMALS {
            tokens_bought = DECIMALS;
            // recompute eth_sold to match exactly that many tokens:
            eth_sold = tokens_bought
                .checked_mul(price_wei)
                .unwrap()
                / DECIMALS;
            // clamp back into 1–50 ETH if needed
            eth_sold = eth_sold.clamp(MIN_ETH, MAX_ETH);
        }

        // fee & total
        let eth_fee = eth_sold / 100;           // 1%
        let total_eth = eth_sold + eth_fee;

        // new total supply
        let total_supply = orig_supply + tokens_bought;

        // fetch existing balance and update
        let bal = sqlx::query!(
            r#"
            SELECT holdings_value 
            FROM token_balance 
            WHERE account_id = $1 AND token_id = $2
            "#,
            a.id,
            t.id
        )
        .fetch_optional(pool)
        .await?;
        let existing_balance: u128 = bal
            .and_then(|r| r.holdings_value.to_string().parse().ok())
            .unwrap_or(0);
        let buyer_token_balance = existing_balance + tokens_bought;

        // timestamps & block numbers strictly increasing
        let timestamp = base_ts + (i as u64 * 300);
        let block_number = 7_483_288 + i as u64;

        // random 32-byte hex IDs
        let mut make_hex = || {
            let mut b = [0u8; 32];
            rng.fill(&mut b);
            format!("0x{}", hex::encode(b))
        };
        let outer_id = make_hex();
        let inner_id = make_hex();
        let tx_hash = make_hex();

        // build JSON
        let event = json!({
            "id": outer_id,
            "event_type": "CultTokenBuy",
            "data": {
                "block_number": block_number,
                "block_timestamp": timestamp,
                "eth_sold": eth_sold.to_string(),
                "eth_fee": eth_fee.to_string(),
                "id": inner_id,
                "marketType": 2,
                "order_referrer": order_referrer,
                "recipient_id": a.id,
                "timestamp": timestamp,
                "token_id": t.id,
                "tokens_bought": tokens_bought.to_string(),
                "total_eth": total_eth.to_string(),
                "total_supply": total_supply.to_string(),
                "market_type": 0,
                "buyer_token_balance": buyer_token_balance.to_string(),
                "trader_id": a.id,
                "transaction_hash": tx_hash,
                "chain_id": 10143
            },
            "timestamp": timestamp
        });

        events.push(event);
    }

    Ok(events)
}

#[tokio::main]
async fn main() -> Result<()> {
    // load .env and init logger
    dotenv().ok();
    env_logger::init();

    // config
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let webhook_secret = env::var("WEBHOOK_SECRET")
        .expect("WEBHOOK_SECRET must be set");
    let api_endpoint = env::var("API_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:8080/webhook".into());
    let event_count = 200;

    println!("Connecting to database…");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Generating {} buy events…", event_count);
    let buy_events = generate_buy_event_json(&pool, event_count).await?;

    // save them to a file
    let path = std::env::current_dir()?.join("token_buy_events.json");
    let mut file = File::create(&path)?;
    file.write_all(serde_json::to_string_pretty(&buy_events)?.as_bytes())?;
    println!("Events written to {}", path.display());

    // send in batches of 0–40, waiting 20–200ms between batches
    let client = Client::new();
    let mut rng = thread_rng();
    let total = buy_events.len();
    let mut idx = 0;

    while idx < total {
        let remaining = total - idx;
        let batch_size = rng.gen_range(5..=30).min(remaining);

        if batch_size > 0 {
            let batch = &buy_events[idx .. idx + batch_size];
            for ev in batch {
                let res = client
                    .post(&api_endpoint)
                    .header("X-Webhook-Signature", &webhook_secret)
                    .header("Content-Type", "application/json")
                    .json(ev)
                    .send()
                    .await?;
                println!("Sent {} → {}", ev["id"], res.status());
            }
            idx += batch_size;
        }

        // pause 20–200 ms before next batch
        let pause = rng.gen_range(20..=200);
        sleep(Duration::from_millis(pause)).await;
    }

    Ok(())
}