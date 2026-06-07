use anyhow::{Result, Context, anyhow};
use chrono::Utc;
use sqlx::{Postgres, Transaction, PgPool};
use serde::{Serialize, Deserialize};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use std::fs::File;
use std::io::BufReader;
use serde_json::from_reader;

// Import your existing modules
use crate::community_airdrops::handler;
use crate::models::Community;
use crate::utils::update_contract_merkle_roots::update_merkle_roots;

// Possible states for operation tracking
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum CommunityUpdateState {
    Pending,
    InvalidatingOldRoots,
    OldRootsInvalidated,
    ProcessingCommunities,
    UpdatingNewRoots,
    Completed,
    Failed,
}

// Define the structure for tracking a batch update operation
#[derive(Debug, Serialize, Deserialize)]
struct CommunityUpdateOperation {
    operation_id: String,
    state: CommunityUpdateState,
    old_roots: Vec<(String, Vec<u8>, i32)>, // (address, merkle_root, holder_count)
    new_roots: Vec<(String, Vec<u8>, i32)>, // (address, merkle_root, holder_count)
    config_hash: String,
    error_message: Option<String>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

// Schema for operation tracking (create this table)
// CREATE TABLE community_update_operations (
//     id SERIAL PRIMARY KEY,
//     operation_id TEXT NOT NULL UNIQUE,
//     state TEXT NOT NULL,
//     old_roots JSONB,
//     new_roots JSONB,
//     config_hash TEXT NOT NULL,
//     error_message TEXT,
//     created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
//     updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
// );

/// Updates all communities with atomic operations and retry logic
pub async fn update_all_communities(
    pool: &PgPool, 
    api_key: &str,
    max_retries: u32
) -> Result<(), anyhow::Error> {
    println!("Starting community update operation");
    
    // Generate a unique operation ID
    let operation_id = format!("community_update_{}", Utc::now().timestamp_millis());
    
    // Load the community configurations
    let base_path = std::env::current_dir()?;
    let full_path = base_path.join("src").join("community_airdrops").join("communities.json");
    let file = File::open(&full_path)?;
    let reader = BufReader::new(file);
    let configs: Vec<CommunityConfig> = from_reader(reader)?;
    
    // Generate a hash of the configs to detect changes
    let config_json = serde_json::to_string(&configs)?;
    let config_hash = format!("{:x}", md5::compute(config_json.as_bytes()));
    
    // Begin initial transaction to check existing state
    let mut tx = pool.begin().await?;
    
    // Check if there's an existing operation with the same config hash
    let existing_op = sqlx::query!(
        r#"
        SELECT operation_id, state FROM community_update_operations 
        WHERE config_hash = $1 
        ORDER BY created_at DESC LIMIT 1
        "#,
        &config_hash
    )
    .fetch_optional(&mut tx)
    .await?;
    
    if let Some(op) = existing_op {
        let state: CommunityUpdateState = op.state.parse()?;
        if state != CommunityUpdateState::Completed && state != CommunityUpdateState::Failed {
            println!("Found existing operation {} in state {:?}, resuming", op.operation_id, state);
            tx.commit().await?;
            return continue_operation(pool, &op.operation_id, state, api_key, max_retries).await;
        }
    }
    
    // Get all existing communities with their merkle roots
    let existing_communities = sqlx::query!(
        r#"
        SELECT id, address, merkle_root, holder_count 
        FROM communities
        WHERE merkle_root IS NOT NULL
        "#
    )
    .fetch_all(&mut tx)
    .await?;
    
    // Create a record of old roots
    let old_roots: Vec<(String, Vec<u8>, i32)> = existing_communities
        .iter()
        .filter_map(|c| {
            if let Some(root) = &c.merkle_root {
                Some((c.address.clone(), root.clone(), c.holder_count))
            } else {
                None
            }
        })
        .collect();
    
    // Create a new operation record
    sqlx::query!(
        r#"
        INSERT INTO community_update_operations
            (operation_id, state, old_roots, new_roots, config_hash, created_at, updated_at)
        VALUES
            ($1, $2, $3, $4, $5, $6, $6)
        "#,
        &operation_id,
        CommunityUpdateState::Pending.to_string(),
        serde_json::to_value(&old_roots)?,
        serde_json::Value::Null,
        &config_hash,
        Utc::now()
    )
    .execute(&mut tx)
    .await?;
    
    tx.commit().await?;
    
    // Start the operation state machine
    continue_operation(pool, &operation_id, CommunityUpdateState::Pending, api_key, max_retries).await
}

/// Continues the operation from a given state with retries
async fn continue_operation(
    pool: &PgPool,
    operation_id: &str,
    initial_state: CommunityUpdateState,
    api_key: &str,
    max_retries: u32
) -> Result<()> {
    let mut current_state = initial_state;
    let mut attempt = 0;
    
    while current_state != CommunityUpdateState::Completed && 
          current_state != CommunityUpdateState::Failed && 
          attempt <= max_retries {
        
        if attempt > 0 {
            // Exponential backoff for retries
            let delay = Duration::from_secs(2u64.pow(attempt.min(6)));
            println!("Retry attempt {} for operation {}. Waiting {:?}", attempt, operation_id, delay);
            sleep(delay).await;
        }
        
        attempt += 1;
        
        match current_state {
            CommunityUpdateState::Pending => {
                current_state = match invalidate_old_roots(pool, operation_id).await {
                    Ok(()) => CommunityUpdateState::OldRootsInvalidated,
                    Err(e) if attempt <= max_retries => {
                        log_error(pool, operation_id, "Failed to invalidate old roots", &e).await?;
                        CommunityUpdateState::InvalidatingOldRoots
                    },
                    Err(e) => {
                        log_error(pool, operation_id, "Failed to invalidate old roots after max retries", &e).await?;
                        CommunityUpdateState::Failed
                    }
                };
            },
            CommunityUpdateState::InvalidatingOldRoots => {
                current_state = match invalidate_old_roots(pool, operation_id).await {
                    Ok(()) => CommunityUpdateState::OldRootsInvalidated,
                    Err(e) if attempt <= max_retries => {
                        log_error(pool, operation_id, "Failed to invalidate old roots (retry)", &e).await?;
                        CommunityUpdateState::InvalidatingOldRoots
                    },
                    Err(e) => {
                        log_error(pool, operation_id, "Failed to invalidate old roots after max retries", &e).await?;
                        CommunityUpdateState::Failed
                    }
                };
            },
            CommunityUpdateState::OldRootsInvalidated => {
                current_state = match process_communities(pool, operation_id, api_key).await {
                    Ok(()) => CommunityUpdateState::UpdatingNewRoots,
                    Err(e) if attempt <= max_retries => {
                        log_error(pool, operation_id, "Failed to process communities", &e).await?;
                        CommunityUpdateState::ProcessingCommunities
                    },
                    Err(e) => {
                        log_error(pool, operation_id, "Failed to process communities after max retries", &e).await?;
                        CommunityUpdateState::Failed
                    }
                };
            },
            CommunityUpdateState::ProcessingCommunities => {
                current_state = match process_communities(pool, operation_id, api_key).await {
                    Ok(()) => CommunityUpdateState::UpdatingNewRoots,
                    Err(e) if attempt <= max_retries => {
                        log_error(pool, operation_id, "Failed to process communities (retry)", &e).await?;
                        CommunityUpdateState::ProcessingCommunities
                    },
                    Err(e) => {
                        log_error(pool, operation_id, "Failed to process communities after max retries", &e).await?;
                        CommunityUpdateState::Failed
                    }
                };
            },
            CommunityUpdateState::UpdatingNewRoots => {
                current_state = match update_new_roots(pool, operation_id).await {
                    Ok(()) => CommunityUpdateState::Completed,
                    Err(e) if attempt <= max_retries => {
                        log_error(pool, operation_id, "Failed to update new roots in contract", &e).await?;
                        CommunityUpdateState::UpdatingNewRoots
                    },
                    Err(e) => {
                        log_error(pool, operation_id, "Failed to update new roots after max retries", &e).await?;
                        CommunityUpdateState::Failed
                    }
                };
            },
            _ => break // Exit for Completed or Failed states
        }
        
        // Update operation state in database
        sqlx::query!(
            r#"
            UPDATE community_update_operations 
            SET state = $1, updated_at = $2
            WHERE operation_id = $3
            "#,
            current_state.to_string(),
            Utc::now(),
            operation_id
        )
        .execute(pool)
        .await?;
    }
    
    if current_state == CommunityUpdateState::Failed {
        Err(anyhow!("Operation failed after {} attempts", attempt))
    } else {
        Ok(())
    }
}

/// Invalidates all old merkle roots in the contract
async fn invalidate_old_roots(
    pool: &PgPool,
    operation_id: &str,
) -> Result<()> {
    println!("Invalidating old merkle roots in contract");
    
    // Get operation data
    let op = sqlx::query!(
        r#"
        SELECT old_roots FROM community_update_operations 
        WHERE operation_id = $1
        "#,
        operation_id
    )
    .fetch_one(pool)
    .await?;
    
    let old_roots: Vec<(String, Vec<u8>, i32)> = serde_json::from_value(op.old_roots.unwrap_or_default())?;
    
    if !old_roots.is_empty() {
        // Prepare batch of merkle roots to invalidate
        let mut merkle_roots = Vec::new();
        let mut holder_counts = Vec::new();
        
        for (_, root, _) in old_roots {
            let root_hex = format!("0x{}", hex::encode(&root));
            merkle_roots.push(root_hex);
            holder_counts.push(0); // Set holder_count to 0
        }
        
        // Send batch update to contract
        update_merkle_roots(merkle_roots, holder_counts)
            .await
            .context("Failed to invalidate old merkle roots in contract")?;
    }
    
    Ok(())
}

/// Process all communities - update database with new merkle data
async fn process_communities(
    pool: &PgPool,
    operation_id: &str,
    api_key: &str,
) -> Result<()> {
    println!("Processing communities");
    
    // Load the community configurations
    let base_path = std::env::current_dir()?;
    let full_path = base_path.join("src").join("community_airdrops").join("communities.json");
    let file = File::open(&full_path)?;
    let reader = BufReader::new(file);
    let configs: Vec<CommunityConfig> = from_reader(reader)?;
    
    let mut new_roots = Vec::new();
    
    for config in configs {
        // Begin transaction for this community
        let mut tx = pool.begin().await?;
        
        // Fetch existing community with its ID
        let existing_community = handler::get_community_by_address(&mut tx, &config.address).await?;
        let start_time = Instant::now();
        
        // Fetch current NFT holders from blockchain
        let owners = fetch_nft_holders(&config, api_key).await?;
        
        // Create temporary community to calculate Merkle data
        let mut temp_community = Community::new(
            config.name.clone(),
            config.img_url.clone(),
            config.address.clone(),
            config.chain.clone()
        ).map_err(|e| anyhow!("Community creation error: {}", e))?;
        
        temp_community.set_merkle_data(owners.clone())
            .map_err(|e| anyhow!("Merkle data calculation failed: {}", e))?;
        
        let duration = start_time.elapsed();
        println!("Processing {} took: {:?}", config.name, duration); 
        
        // Update or create community in database
        let db_community = match existing_community {
            Some(existing) => {
                handler::update_community(
                    &mut tx,
                    &existing.address,
                    temp_community.merkle_root.as_ref(),
                    temp_community.merkle_proofs.as_ref(),
                    temp_community.last_updated_time,
                    owners.length() as i32
                ).await?
            }
            None => {
                handler::create_community(
                    &mut tx,
                    &temp_community.name,
                    &temp_community.img_url,
                    &temp_community.address,
                    &temp_community.chain,
                    owners.length() as u32,                      
                    temp_community.merkle_root.as_ref(),
                    temp_community.merkle_proofs.as_ref(),
                    temp_community.last_updated_time,
                ).await?
            }
        };
        
        println!("Updated community: {}", db_community.name);

        // Generate dummy accounts if needed
        handler::generate_dummy_account_data(
            &mut tx,
            &owners
        ).await?;

        // Refresh community memberships
        handler::refresh_community_memberships(
            &mut tx,
            db_community.id,
            &owners
        ).await?;
        
        // Store the new merkle root for contract update
        if let Some(root) = &temp_community.merkle_root {
            new_roots.push((
                db_community.address.clone(),
                root.clone(),
                owners.length() as i32
            ));
        }
        
        // Commit the transaction for this community
        tx.commit().await?;
    }
    
    // Update operation with new roots
    sqlx::query!(
        r#"
        UPDATE community_update_operations 
        SET new_roots = $1
        WHERE operation_id = $2
        "#,
        serde_json::to_value(&new_roots)?,
        operation_id
    )
    .execute(pool)
    .await?;
    
    Ok(())
}

/// Update contract with new merkle roots
async fn update_new_roots(
    pool: &PgPool,
    operation_id: &str,
) -> Result<()> {
    println!("Updating contract with new merkle roots");
    
    // Get operation data with new roots
    let op = sqlx::query!(
        r#"
        SELECT new_roots FROM community_update_operations 
        WHERE operation_id = $1
        "#,
        operation_id
    )
    .fetch_one(pool)
    .await?;
    
    let new_roots: Vec<(String, Vec<u8>, i32)> = serde_json::from_value(op.new_roots.unwrap_or_default())?;
    
    if !new_roots.is_empty() {
        // Prepare batch of merkle roots to update
        let mut merkle_roots = Vec::new();
        let mut holder_counts = Vec::new();
        
        for (_, root, holder_count) in new_roots {
            let root_hex = format!("0x{}", hex::encode(&root));
            merkle_roots.push(root_hex);
            holder_counts.push(holder_count as u32);
        }
        
        // Send batch update to contract
        update_merkle_roots(merkle_roots, holder_counts)
            .await
            .context("Failed to update new merkle roots in contract")?;
    }
    
    Ok(())
}

/// Logs error details to operation record
async fn log_error(
    pool: &PgPool,
    operation_id: &str,
    context: &str,
    error: &anyhow::Error,
) -> Result<()> {
    let error_message = format!("{}: {}", context, error);
    println!("ERROR: {}", error_message);
    
    sqlx::query!(
        r#"
        UPDATE community_update_operations 
        SET error_message = $1, updated_at = $2
        WHERE operation_id = $3
        "#,
        error_message,
        Utc::now(),
        operation_id
    )
    .execute(pool)
    .await?;
    
    Ok(())
}