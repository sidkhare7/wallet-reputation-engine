use anyhow::{Result, Context, anyhow};
use chrono::Utc;
use sqlx::{Postgres, Transaction, PgPool};
use alloy_primitives::B256;
use alloy_merkle::MerkleTree;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use serde::{Serialize, Deserialize};

// Import the update_contract_merkle_roots function
use crate::utils::update_contract_merkle_roots::update_merkle_roots;

// Constants
const DIAMOND_HAND_ID: &str = "0x000000000000000000000000000000000d1a305d";
const DIAMOND_HAND_NAME: &str = "diamondHandList";
const DIAMOND_HAND_IMG: &str = "https://placeholder.com/diamond_hands.png";
const DIAMOND_HAND_ADDRESS: &str = "0x000000000000000000000000000000000d1a305d";
const DIAMOND_HAND_CHAIN: &str = "monadTestnet";
const SAMPLE_SIZE: i32 = 10; // Holder count for the new merkle root

// Possible states for operation tracking
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum DiamondHandsUpdateState {
    Pending,
    InvalidatingOldRoot,
    OldRootInvalidated,
    UpdatingNewRoot,
    Completed,
    Failed,
}

// Schema for operation tracking (create this table)
// CREATE TABLE diamond_hands_operations (
//     id SERIAL PRIMARY KEY,
//     operation_id TEXT NOT NULL UNIQUE,
//     state TEXT NOT NULL,
//     old_merkle_root BYTEA,
//     new_merkle_root BYTEA,
//     account_ids JSONB,
//     error_message TEXT,
//     updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
// );

/// Updates diamond hands list with retry logic and state tracking
pub async fn update_diamond_hands(
    pool: &PgPool,
    account_ids: Vec<String>,
    max_retries: u32,
) -> Result<()> {
    println!("Starting diamond hands update operation");
    
    // Generate a unique operation ID
    let operation_id = format!("diamond_hands_update_{}", Utc::now().timestamp_millis());
    
    // Begin initial transaction
    let mut tx = pool.begin().await?;

    // Get current root and prepare operation tracking
    let (operation_state, old_root_bytes, old_root_hex) = prepare_operation(&mut tx, &operation_id).await?;
    
    // If this is a retry of a previously started operation
    if operation_state != DiamondHandsUpdateState::Pending {
        println!("Resuming operation in state: {:?}", operation_state);
        tx.commit().await?;
        return continue_operation(pool, &operation_id, operation_state, max_retries).await;
    }
    
    // Generate Merkle data for new list
    let (new_root_bytes, new_root_hex, timestamp, proofs) = 
        if !account_ids.is_empty() {
            generate_merkle_data(&account_ids)?
        } else {
            return Err(anyhow!("Account IDs list cannot be empty"));
        };
    
    // Update operation with the data we've generated
    sqlx::query!(
        r#"
        UPDATE diamond_hands_operations 
        SET new_merkle_root = $1, account_ids = $2
        WHERE operation_id = $3
        "#,
        &new_root_bytes,
        serde_json::to_value(&account_ids)?,
        &operation_id
    )
    .execute(&mut tx)
    .await?;
    
    // Commit the preparation transaction
    tx.commit().await?;
    
    // Now start the multi-phase operation with retries
    continue_operation(
        pool, 
        &operation_id, 
        DiamondHandsUpdateState::Pending, 
        max_retries
    ).await?;
    
    println!("Diamond hands update completed successfully");
    Ok(())
}

/// Prepares the operation by getting the current state and root
async fn prepare_operation(
    tx: &mut Transaction<'_, Postgres>,
    operation_id: &str,
) -> Result<(DiamondHandsUpdateState, Option<Vec<u8>>, Option<String>)> {
    // Check if operation already exists (for retry scenarios)
    let existing_op = sqlx::query!(
        r#"
        SELECT state, old_merkle_root 
        FROM diamond_hands_operations 
        WHERE operation_id = $1
        "#,
        operation_id
    )
    .fetch_optional(&mut **tx)
    .await?;
    
    if let Some(op) = existing_op {
        // Operation exists, return its current state
        let state: DiamondHandsUpdateState = op.state.parse()?;
        let old_root_hex = if let Some(root) = &op.old_merkle_root {
            Some(format!("0x{}", hex::encode(root)))
        } else {
            None
        };
        
        return Ok((state, op.old_merkle_root, old_root_hex));
    }
    
    // New operation - get current merkle root
    let current_root = sqlx::query!(
        r#"
        SELECT merkle_root FROM communities 
        WHERE id = $1
        "#,
        DIAMOND_HAND_ID
    )
    .fetch_optional(&mut **tx)
    .await?;
    
    let old_root_bytes = current_root.and_then(|r| r.merkle_root);
    let old_root_hex = if let Some(root) = &old_root_bytes {
        Some(format!("0x{}", hex::encode(root)))
    } else {
        None
    };
    
    // Create new operation record
    sqlx::query!(
        r#"
        INSERT INTO diamond_hands_operations
            (operation_id, state, old_merkle_root, updated_at)
        VALUES
            ($1, $2, $3, $4)
        "#,
        operation_id,
        DiamondHandsUpdateState::Pending.to_string(),
        old_root_bytes,
        Utc::now()
    )
    .execute(&mut **tx)
    .await?;
    
    Ok((DiamondHandsUpdateState::Pending, old_root_bytes, old_root_hex))
}

/// Generates merkle tree and proofs from account IDs
fn generate_merkle_data(
    account_ids: &[String]
) -> Result<(Vec<u8>, String, chrono::DateTime<Utc>, serde_json::Value)> {
    let mut leaves = Vec::new();
    
    // Validate addresses and create leaves
    for account_id in account_ids {
        let account_bytes = hex::decode(account_id.trim_start_matches("0x"))
            .map_err(|e| anyhow!("Invalid hex in address {}: {}", account_id, e))?;
        
        if account_bytes.len() != 20 {
            return Err(anyhow!("Address {} is not 20 bytes", account_id));
        }

        // Pad address into 32-byte Merkle leaf
        let mut address_bytes = [0u8; 32];
        address_bytes[12..].copy_from_slice(&account_bytes);
        leaves.push(B256::from(address_bytes));
    }

    // Build Merkle tree
    let mut merkle_tree = MerkleTree::new();
    for leaf in &leaves {
        merkle_tree.insert(*leaf);
    }
    merkle_tree.finish();

    // Generate proofs as a map of address -> proof array
    let mut proofs_map = HashMap::new();
    for (i, leaf) in leaves.iter().enumerate() {
        let proof = merkle_tree.create_proof(leaf)
            .ok_or_else(|| anyhow!("Failed to create proof for {}", account_ids[i]))?;

        // Convert proof to string array format
        let proof_path: Vec<String> = proof.siblings
            .iter()
            .map(|node| format!("{:?}", node))
            .collect();
        
        // Use address as key and proof array as value
        proofs_map.insert(account_ids[i].clone(), proof_path);
    }

    // Convert map to JSONB
    let all_proofs = serde_json::to_value(proofs_map)?;
    
    // Create hex representation of merkle root for contract update
    let root_bytes = merkle_tree.root.to_vec();
    let root_hex = format!("0x{}", hex::encode(&root_bytes));
    
    Ok((root_bytes, root_hex, Utc::now(), all_proofs))
}

/// Continues operation from a given state with retries
async fn continue_operation(
    pool: &PgPool,
    operation_id: &str,
    initial_state: DiamondHandsUpdateState,
    max_retries: u32,
) -> Result<()> {
    let mut current_state = initial_state;
    let mut attempt = 0;
    
    while current_state != DiamondHandsUpdateState::Completed && 
          current_state != DiamondHandsUpdateState::Failed && 
          attempt <= max_retries {
        
        if attempt > 0 {
            // Exponential backoff for retries
            let delay = Duration::from_secs(2u64.pow(attempt.min(6)));
            println!("Retry attempt {} for operation {}. Waiting {:?}", attempt, operation_id, delay);
            sleep(delay).await;
        }
        
        attempt += 1;
        
        match current_state {
            DiamondHandsUpdateState::Pending => {
                // Transition to invalidating old root if exists
                current_state = match invalidate_old_root(pool, operation_id).await {
                    Ok(()) => DiamondHandsUpdateState::OldRootInvalidated,
                    Err(e) if attempt <= max_retries => {
                        log_error(pool, operation_id, "Failed to invalidate old root", &e).await?;
                        DiamondHandsUpdateState::InvalidatingOldRoot
                    },
                    Err(e) => {
                        log_error(pool, operation_id, "Failed to invalidate old root after max retries", &e).await?;
                        DiamondHandsUpdateState::Failed
                    }
                };
            },
            DiamondHandsUpdateState::InvalidatingOldRoot => {
                // Retry invalidating old root
                current_state = match invalidate_old_root(pool, operation_id).await {
                    Ok(()) => DiamondHandsUpdateState::OldRootInvalidated,
                    Err(e) if attempt <= max_retries => {
                        log_error(pool, operation_id, "Failed to invalidate old root (retry)", &e).await?;
                        DiamondHandsUpdateState::InvalidatingOldRoot
                    },
                    Err(e) => {
                        log_error(pool, operation_id, "Failed to invalidate old root after max retries", &e).await?;
                        DiamondHandsUpdateState::Failed
                    }
                };
            },
            DiamondHandsUpdateState::OldRootInvalidated => {
                // Update database and register new root with contract
                current_state = match update_database_and_contract(pool, operation_id).await {
                    Ok(()) => DiamondHandsUpdateState::Completed,
                    Err(e) if attempt <= max_retries => {
                        log_error(pool, operation_id, "Failed to update database or contract", &e).await?;
                        DiamondHandsUpdateState::UpdatingNewRoot
                    },
                    Err(e) => {
                        log_error(pool, operation_id, "Failed to update database or contract after max retries", &e).await?;
                        DiamondHandsUpdateState::Failed
                    }
                };
            },
            DiamondHandsUpdateState::UpdatingNewRoot => {
                // Retry database and contract update
                current_state = match update_database_and_contract(pool, operation_id).await {
                    Ok(()) => DiamondHandsUpdateState::Completed,
                    Err(e) if attempt <= max_retries => {
                        log_error(pool, operation_id, "Failed to update database or contract (retry)", &e).await?;
                        DiamondHandsUpdateState::UpdatingNewRoot
                    },
                    Err(e) => {
                        log_error(pool, operation_id, "Failed to update database or contract after max retries", &e).await?;
                        DiamondHandsUpdateState::Failed
                    }
                };
            },
            _ => break // Exit for Completed or Failed states
        }
        
        // Update operation state in database
        sqlx::query!(
            r#"
            UPDATE diamond_hands_operations 
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
    
    if current_state == DiamondHandsUpdateState::Failed {
        Err(anyhow!("Operation failed after {} attempts", attempt))
    } else {
        Ok(())
    }
}

/// Invalidates the old merkle root in the contract
async fn invalidate_old_root(
    pool: &PgPool,
    operation_id: &str,
) -> Result<()> {
    // Get operation data
    let op = sqlx::query!(
        r#"
        SELECT old_merkle_root FROM diamond_hands_operations 
        WHERE operation_id = $1
        "#,
        operation_id
    )
    .fetch_one(pool)
    .await?;
    
    // If there's an old root, set its holder count to 0
    if let Some(root) = op.old_merkle_root {
        let root_hex = format!("0x{}", hex::encode(&root));
        
        println!("Invalidating old root: {}", root_hex);
        update_merkle_roots(
            vec![root_hex], 
            vec![0]  // Set holder_count to 0
        ).await.context("Failed to invalidate old merkle root in contract")?;
    }
    
    Ok(())
}

/// Updates both database and contract with new merkle root data
async fn update_database_and_contract(
    pool: &PgPool, 
    operation_id: &str
) -> Result<()> {
    // Get all operation data
    let op = sqlx::query!(
        r#"
        SELECT new_merkle_root, account_ids FROM diamond_hands_operations 
        WHERE operation_id = $1
        "#,
        operation_id
    )
    .fetch_one(pool)
    .await?;
    
    let new_root = op.new_merkle_root.ok_or_else(|| anyhow!("Missing new merkle root data"))?;
    let account_ids: Vec<String> = serde_json::from_value(op.account_ids.unwrap_or_default())?;
    
    // Regenerate proofs and timestamp for database update
    let (_, new_root_hex, timestamp, proofs) = generate_merkle_data(&account_ids)?;
    
    // Begin transaction for database updates
    let mut tx = pool.begin().await?;
    
    // Clear existing entries in diamond_hand_list
    sqlx::query("TRUNCATE diamond_hand_list")
        .execute(&mut *tx)
        .await?;
    
    // Insert account IDs into diamond_hand_list
    sqlx::query!(
        r#"
        INSERT INTO diamond_hand_list 
            (account_id, last_updated_time)
        SELECT 
            unnest($1::text[]), 
            $2
        "#,
        &account_ids,
        timestamp
    )
    .execute(&mut *tx)
    .await?;

    // Upsert into communities table
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
        DIAMOND_HAND_ID,
        DIAMOND_HAND_NAME,
        DIAMOND_HAND_IMG,
        DIAMOND_HAND_ADDRESS,
        DIAMOND_HAND_CHAIN,
        new_root,
        timestamp,
        proofs,
        SAMPLE_SIZE
    )
    .execute(&mut *tx)
    .await?;
    
    // Commit database changes
    tx.commit().await?;
    
    // Update contract with new merkle root and holder count
    println!("Updating contract with new root: {}", new_root_hex);
    update_merkle_roots(
        vec![new_root_hex], 
        vec![SAMPLE_SIZE as u32]
    ).await.context("Failed to update contract with new merkle root")?;
    
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
        UPDATE diamond_hands_operations 
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