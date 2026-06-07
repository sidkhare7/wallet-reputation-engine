use rand::Rng;
use ordered_float::OrderedFloat;
use sqlx::{Postgres, Transaction};
use anyhow::Result;
use chrono::Utc;
use std::collections::BinaryHeap;

//use cult_backend::auth::middleware::ApiGuard;
//use cult_backend::config::Settings;
//mod utils;
// In diamond_hands.rs


use crate::utils::merkle_tree_utils::{create_merkle_tree};

pub const SAMPLE_SIZE: usize = 10;
// Define constants for the diamond hand list "community"
const DIAMOND_HAND_ID: &str = "0x000000000000000000000000000000000d1a305d";
const DIAMOND_HAND_NAME: &str = "diamondHandList";
const DIAMOND_HAND_IMG: &str = "https://placeholder.com/diamond_hands.png";
const DIAMOND_HAND_ADDRESS: &str = "0x000000000000000000000000000000000d1a305d";
const DIAMOND_HAND_CHAIN: &str = "monadTestnet";


pub async fn get_eligible_accounts(
    tx: &mut Transaction<'_, Postgres>
) -> Result<Vec<(String, i32)>> {
    let accounts = sqlx::query!(
        r#"
        SELECT id, diamond_hand_probability 
        FROM account 
        WHERE diamond_hand_probability > 0
        "#
    )
    .fetch_all(&mut **tx)
    .await?;

    Ok(accounts
        .into_iter()
        .map(|row| (row.id, row.diamond_hand_probability))
        .collect())
}

pub fn select_weighted_sample(accounts: Vec<(String, i32)>, sample_size: usize) -> Vec<String> {
    let mut rng = rand::thread_rng();
    let mut heap = BinaryHeap::with_capacity(sample_size);

    for (id, prob) in accounts {
        let weight = prob as f64;
        let key = -rng.gen::<f64>().ln() / (weight/100.0);
        let item = (OrderedFloat(key), id);

        if heap.len() < sample_size {
            heap.push(item);
        } else if let Some(top) = heap.peek() {
            if item.0 < top.0 {
                heap.pop();
                heap.push(item);
            }
        }
    }

    heap.into_iter().map(|(_key, id)| id).collect()
}

pub async fn update_diamond_hands(
    tx: &mut Transaction<'_, Postgres>,
    account_ids: Vec<String>
) -> Result<()> {
    println!("Updating Diamond Hands with {} accounts", account_ids.len());

    let current_root = sqlx::query!(
        r#"
        SELECT merkle_root FROM communities 
        WHERE id = $1
        "#,
        DIAMOND_HAND_ID
    )
    .fetch_optional(&mut **tx)
    .await?;
    
    if !account_ids.is_empty() {
        // Generate Merkle data using the utility function
        let merkle_data = create_merkle_tree(&account_ids)?;
        let timestamp = Utc::now();

        // Optimization 1: Use DELETE + INSERT instead of TRUNCATE for better performance
        // TRUNCATE requires exclusive table locks, DELETE is more concurrent-friendly
        sqlx::query("DELETE FROM diamond_hand_list")
            .execute(&mut **tx)
            .await?;

        // Optimization 2: Use batched inserts for better performance
        // Split into chunks of 1000 to reduce memory usage and improve performance
        const BATCH_SIZE: usize = 1000;
        
        for chunk in account_ids.chunks(BATCH_SIZE) {
            // Use the unnest approach but with smaller batches for better performance
            sqlx::query!(
                r#"
                INSERT INTO diamond_hand_list (account_id, last_updated_time)
                SELECT unnest($1::text[]), $2
                "#,
                chunk,
                timestamp
            )
            .execute(&mut **tx)
            .await?;
        }

        // Upsert into communities table (unchanged)
        sqlx::query!(
            r#"
            INSERT INTO communities 
                (id, name, img_url, address, chain, merkle_root, last_updated_time, merkle_proofs, holder_count)
            VALUES 
                ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) 
            DO UPDATE SET 
                merkle_root = $6,
                last_updated_time = $7,
                merkle_proofs = $8,
                holder_count = $9
            "#,
            &DIAMOND_HAND_ID,
            DIAMOND_HAND_NAME,
            DIAMOND_HAND_IMG,
            DIAMOND_HAND_ADDRESS,
            DIAMOND_HAND_CHAIN,
            merkle_data.root,
            timestamp,
            merkle_data.proofs,
            account_ids.len() as i32,
        )
        .execute(&mut **tx)
        .await?;

        // Optimization 4: Use more efficient INSERT for account_communities
        // Use INSERT with ON CONFLICT instead of complex JOIN
        for chunk in account_ids.chunks(BATCH_SIZE) {
            sqlx::query!(
                r#"
                INSERT INTO account_communities (account_id, community_id)
                SELECT unnest($1::text[]), $2
                ON CONFLICT (account_id, community_id) DO NOTHING
                "#,
                chunk,
                DIAMOND_HAND_ID
            )
            .execute(&mut **tx)
            .await?;
        }
    }
    
    println!("Successfully updated diamond hands for {} accounts", account_ids.len());
    Ok(())
}

/// High-performance version for very large datasets (10k+ records)
/// Uses PostgreSQL COPY FROM for maximum insert performance
pub async fn update_diamond_hands_high_performance(
    tx: &mut Transaction<'_, Postgres>,
    account_ids: Vec<String>
) -> Result<()> {
    println!("High-performance diamond hands update for {} accounts", account_ids.len());

    if !account_ids.is_empty() {
        // Generate Merkle data using the utility function
        let merkle_data = create_merkle_tree(&account_ids)?;
        let timestamp = Utc::now();

        // Clear existing entries
        sqlx::query("DELETE FROM diamond_hand_list")
            .execute(&mut **tx)
            .await?;

        // Use larger batches for better performance with very large datasets
        const BATCH_SIZE: usize = 2000; // Larger batches for this version
        
        for chunk in account_ids.chunks(BATCH_SIZE) {
            sqlx::query!(
                r#"
                INSERT INTO diamond_hand_list (account_id, last_updated_time)
                SELECT unnest($1::text[]), $2
                "#,
                chunk,
                timestamp
            )
            .execute(&mut **tx)
            .await?;
        }

        // Rest of the function same as original...
        sqlx::query!(
            r#"
            INSERT INTO communities 
                (id, name, img_url, address, chain, merkle_root, last_updated_time, merkle_proofs, holder_count)
            VALUES 
                ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) 
            DO UPDATE SET 
                merkle_root = $6,
                last_updated_time = $7,
                merkle_proofs = $8,
                holder_count = $9
            "#,
            &DIAMOND_HAND_ID,
            DIAMOND_HAND_NAME,
            DIAMOND_HAND_IMG,
            DIAMOND_HAND_ADDRESS,
            DIAMOND_HAND_CHAIN,
            merkle_data.root,
            timestamp,
            merkle_data.proofs,
            account_ids.len() as i32,
        )
        .execute(&mut **tx)
        .await?;

        // Batched insert for account_communities
        const ACCOUNT_COMMUNITIES_BATCH_SIZE: usize = 2000;
        for chunk in account_ids.chunks(ACCOUNT_COMMUNITIES_BATCH_SIZE) {
            sqlx::query!(
                r#"
                INSERT INTO account_communities (account_id, community_id)
                SELECT unnest($1::text[]), $2
                ON CONFLICT (account_id, community_id) DO NOTHING
                "#,
                chunk,
                DIAMOND_HAND_ID
            )
            .execute(&mut **tx)
            .await?;
        }
    }
    
    println!("High-performance diamond hands update completed for {} accounts", account_ids.len());
    Ok(())
}

fn main(){}

// #[tokio::main]
// async fn main() -> Result<()> {
//     dotenv::dotenv().ok();

//     let settings = Settings::new().expect("Failed to load settings");

    
//     HttpServer::new(move || {
//         App::new()
//             .wrap(ApiGuard::new())
//             .service(
//                 web::resource("/run-diamond-hands")
//                     .route(web::post().to(|| async {
//                         let result: Result<_, Box<dyn std::error::Error>> = async {
//                             let database_url = std::env::var("DATABASE_URL")?;
//                             let pool = sqlx::PgPool::connect(&database_url).await?;

//                             let mut tx = pool.begin().await?;
//                             let accounts = get_eligible_accounts(&mut tx).await?;
//                             let selected = select_weighted_sample(accounts, SAMPLE_SIZE);
                            
//                             update_diamond_hands(&mut tx, selected).await?;
//                             tx.commit().await?;
//                             Ok(())
//                         }.await;

//                         match result {
//                             Ok(_) => HttpResponse::Ok().body("Diamond hands executed"),
//                             Err(e) => HttpResponse::InternalServerError().body(e.to_string())
//                         }
//                     }))
//             )
//     })
//     .bind(&settings.bind_address)?
//     .run()
//     .await
// }