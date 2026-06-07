use actix_web::{web, HttpResponse, Error};
use sqlx::{PgPool, Postgres, Transaction};
use log::{info, error};
use serde_json::{json};
use sqlx::types::BigDecimal;
use std::str::FromStr;

/// Updates all community scores based on the diamond hand probability of their member accounts
/// Returns HTTP response with the results
pub async fn update_all_community_scores(
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    // Start by getting all communities
    let communities = match sqlx::query!(
        "SELECT id, name FROM communities"
    )
    .fetch_all(pool.get_ref())
    .await
    {
        Ok(communities) => communities,
        Err(e) => {
            error!("Failed to fetch communities: {}", e);
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to fetch communities",
                "details": e.to_string()
            })));
        }
    };
    
    info!("Updating scores for {} communities", communities.len());
    
    let mut updated_count = 0;
    let mut failed_communities = Vec::new();
    
    // Process each community
    for community in &communities {
        // Start a new transaction for each community
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                error!("Failed to start transaction: {}", e);
                failed_communities.push((community.id.clone(), e.to_string()));
                continue;
            }
        };
        
        // Calculate score within the transaction
        let score = match calculate_community_score(&mut tx, &community.id).await {
            Ok(score) => score,
            Err(e) => {
                error!("Failed to calculate score for community {}: {}", community.id, e);
                let _ = tx.rollback().await; // Ignore rollback errors
                failed_communities.push((community.id.clone(), e.to_string()));
                continue;
            }
        };
        
        // Update the community's score
        match sqlx::query!(
            "UPDATE communities SET community_score = $1, last_updated_time = NOW() WHERE id = $2",
            score,
            community.id
        )
        .execute(&mut *tx)  // Use *tx to reference the executor
        .await
        {
            Ok(_) => {
                info!("Updated community '{}' with score {}", community.name, score);
            },
            Err(e) => {
                error!("Failed to update score for community {}: {}", community.id, e);
                let _ = tx.rollback().await; // Ignore rollback errors
                failed_communities.push((community.id.clone(), e.to_string()));
                continue;
            }
        };
        
        // Commit the transaction
        if let Err(e) = tx.commit().await {
            error!("Failed to commit transaction for community {}: {}", community.id, e);
            failed_communities.push((community.id.clone(), e.to_string()));
            continue;
        }
        
        updated_count += 1;
    }
    
    // Prepare the response
    if failed_communities.is_empty() {
        Ok(HttpResponse::Ok().json(json!({
            "success": true,
            "message": format!("Successfully updated scores for all {} communities", updated_count)
        })))
    } else {
        Ok(HttpResponse::Ok().json(json!({
            "success": true,
            "message": format!("Updated scores for {} out of {} communities", updated_count, communities.len()),
            "failed_communities": failed_communities
        })))
    }
}

/// Calculates the community score by averaging diamond hand probabilities of all accounts
/// Formula: community_score = (avg_diamond_hand_probability / MAX_DIAMOND_HAND) * 100
pub async fn calculate_community_score(
    tx: &mut Transaction<'_, Postgres>, 
    community_id: &str
) -> Result<BigDecimal, sqlx::Error> {
    // Maximum diamond hand probability is 2% (stored as 200 basis points in the database)
    let max_probability = BigDecimal::from_str("500.0").unwrap();
    let hundred = BigDecimal::from_str("100.0").unwrap();
    let zero = BigDecimal::from_str("0.0").unwrap();
    
    // Get the average diamond_hand_probability directly using a JOIN
    let row = sqlx::query!(
        "SELECT AVG(a.diamond_hand_probability) as avg_probability
         FROM account a
         JOIN account_communities ac ON a.id = ac.account_id
         WHERE ac.community_id = $1",
        community_id
    )
    .fetch_one(&mut **tx)
    .await?;
    
    // If no accounts found or AVG returns null, use 0
    let average_probability = match row.avg_probability {
        Some(val) => val,
        None => BigDecimal::from(0)
    };
    
    // Calculate score: (average_probability / max_probability) * 100
    let score = (&average_probability / &max_probability) * &hundred;
    
    // Ensure the score is within 0-100 range
    let score = if score < zero {
        zero
    } else if score > hundred {
        hundred.clone()
    } else {
        score
    };
    
    Ok(score)
}

/// Updates a single community's score
/// Useful for updating scores on-demand through an API endpoint
pub async fn update_single_community_score(
    pool: web::Data<PgPool>,
    community_id: web::Path<String>
) -> Result<HttpResponse, Error> {
    // Start a transaction
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            error!("Failed to start transaction: {}", e);
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Database error",
                "details": e.to_string()
            })));
        }
    };
    
    // Verify community exists
    let community = match sqlx::query!(
        "SELECT name FROM communities WHERE id = $1",
        community_id.as_str()
    )
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(community)) => community,
        Ok(None) => {
            let _ = tx.rollback().await;
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Community not found",
                "community_id": community_id.as_str()
            })));
        },
        Err(e) => {
            error!("Failed to fetch community {}: {}", community_id, e);
            let _ = tx.rollback().await;
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Database error",
                "details": e.to_string()
            })));
        }
    };
    
    // Calculate the score
    let score = match calculate_community_score(&mut tx, community_id.as_str()).await {
        Ok(score) => score,
        Err(e) => {
            error!("Failed to calculate score for community {}: {}", community_id, e);
            let _ = tx.rollback().await;
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to calculate community score",
                "details": e.to_string()
            })));
        }
    };
    
    // Update the community's score
    match sqlx::query!(
        "UPDATE communities SET community_score = $1, last_updated_time = NOW() WHERE id = $2",
        score,
        community_id.as_str()
    )
    .execute(&mut *tx)
    .await
    {
        Ok(_) => {
            info!("Updated community '{}' with score {}", community.name, score);
        },
        Err(e) => {
            error!("Failed to update score for community {}: {}", community_id, e);
            let _ = tx.rollback().await;
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to update community score",
                "details": e.to_string()
            })));
        }
    };
    
    // Commit the transaction
    if let Err(e) = tx.commit().await {
        error!("Failed to commit transaction for community {}: {}", community_id, e);
        return Ok(HttpResponse::InternalServerError().json(json!({
            "error": "Failed to finalize score update",
            "details": e.to_string()
        })));
    }
    
    // Return success response
    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "community_id": community_id.as_str(),
        "community_name": community.name,
        "community_score": score.to_string(), // Convert BigDecimal to string for JSON
        "message": "Community score updated successfully"
    })))
}