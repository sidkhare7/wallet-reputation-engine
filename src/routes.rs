use actix_web::{get, post, web, HttpResponse, Responder};
use sqlx::{PgPool, Postgres, Transaction};
use crate::models::{ UpcomingTokensResponse, MerkleProofResponse, CultTokenTopHolder, CultTokenDataResponse, CommunityResponse, UpdateAccountRequest, WatchlistActionRequest, CreateAccountRequest, Account, CultTokensResponse, TokenTradesResponse,AccountData, Community, CreateAccountResponse, DiamondHandsRequest, DiamondHandsResponse};
use std::result::Result::Ok;
use rand::Rng;
use anyhow;
use utoipa::OpenApi;
use bigdecimal::ToPrimitive;

fn generate_referral_code() -> String {
    // Characters to use in the referral code (alphanumeric without confusing characters)
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    const CODE_LENGTH: usize = 8;
    
    let mut rng = rand::rng();
    let code: String = (0..CODE_LENGTH)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    
    code
}


async fn validate_referral_code(
    tx: &mut Transaction<'_, Postgres>,
    referral_code: &str
) -> Result<String, anyhow::Error> {
    // Get referrer info
    let record = sqlx::query!(
        "SELECT id, total_referrals FROM account WHERE referral_code = $1",
        referral_code
    )
    .fetch_optional(&mut **tx)  
    .await?;
    
    match record {
        Some(r) if r.total_referrals >= Some(5) => {
            Err(anyhow::anyhow!("Referral code has reached maximum usage (5)"))
        },
        Some(r) => Ok(r.id),
        None => Err(anyhow::anyhow!("Invalid referral code"))
    }
}

/////////////////////
/// ACCOUNT STUFF ////
/// //////////////////

/// Create a new user account
#[utoipa::path(
    tag = "Account",
    post,
    path = "/account",
    request_body(
        content = CreateAccountRequest,
        description = "Payload for creating a new account, including optional referral and social handles",
        content_type = "application/json"
    ),
    responses(
        (status = 201, description = "Account successfully created", body = CreateAccountResponse),
        (status = 400, description = "Invalid referral code"),
        (status = 409, description = "Account already exists"),
        (status = 500, description = "Internal server or database error")
    )
)]
#[post("/account")]
pub async fn create_account(
    pool: web::Data<PgPool>,
    request: web::Json<CreateAccountRequest>,
) -> impl Responder {

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Failed to start transaction: {e}")),
    };

    // Check for existing account
    match sqlx::query!("SELECT id FROM account WHERE id = $1", request.user_id)
        .fetch_optional(&mut *tx)
        .await
    {
        Ok(Some(_)) => return HttpResponse::Conflict().json("Account already exists"),
        Err(e) => return HttpResponse::InternalServerError().json(format!("Database error: {e}")),
        _ => (),
    };
    // Handle referral code validation
    let referrer_id = match &request.referral_code {
        Some(code) => match validate_referral_code(&mut tx, code).await {
            Ok(id) => Some(id),
            Err(e) => {
                // Check if it's specifically a max usage error for better user feedback
                if e.to_string().contains("maximum usage") {
                    return HttpResponse::BadRequest().json("Referral code has reached maximum usage limit of 5");
                } else {
                    return HttpResponse::BadRequest().json(format!("Invalid referral code: {e}"));
                }
            },
        },
        None => None,
    };

    
    
    // Generate unique referral code with collision check
    let mut new_referral_code;
    loop {
        new_referral_code = generate_referral_code();
        match validate_referral_code(&mut tx, &new_referral_code).await {
            Ok(_) => continue, // Code exists, regenerate
            Err(_) => break,   // Unique code found
        }
    }
    

    // Insert new account
    match sqlx::query!(
        r#"
        INSERT INTO account (
            id, slug, referral_code, diamond_hand_probability, 
            referrer_id, total_referrals, fee_collected, twitter, discord
        )
        VALUES ($1, $2, $3, 0, $4, 0, 0, $5, $6)
        "#,
        request.user_id,
        None::<String>,
        new_referral_code,
        referrer_id,
        request.twitter.as_ref(),
        request.discord.as_ref()
    )
    .execute(&mut *tx)
    .await
    {
        Ok(_) => (),
        Err(e) => return HttpResponse::InternalServerError().body(format!("Failed to create account: {e}")),
    };

    
    
    // Update referrer's count
    if let Some(ref_id) = referrer_id {
        if let Err(e) = sqlx::query!(
            "UPDATE account SET total_referrals = total_referrals + 1 WHERE id = $1",
            ref_id
        )
        .execute(&mut *tx)
        .await
        {
            return HttpResponse::InternalServerError().body(format!("Failed to update referrals: {e}"));
        }
    }
    
    // Commit transaction
    if let Err(e) = tx.commit().await {
        return HttpResponse::InternalServerError().body(format!("Transaction commit failed: {e}"));
    }

    
    HttpResponse::Created().json(serde_json::json!({
        "user_id": request.user_id,
        "referral_code": new_referral_code,
    }))
}

// Get user account
#[utoipa::path(
    tag = "Account",
    get,
    path = "/account/{user_id}",
    params(
        ("user_id" = String, Path, description = "User identifier")
    ),
    responses(
        (status = 200, description = "Returns basic account data", body = Account),
        (status = 404, description = "Account not found"),
        (status = 500, description = "Database error")
    )
)]
#[get("/account/{user_id}")]
pub async fn get_account(pool: web::Data<PgPool>, user_id: web::Path<String>) -> impl Responder {
    let result = sqlx::query!(
        r#"
        SELECT 
            id, slug, referral_code, diamond_hand_probability, reputation,
            referrer_id, total_referrals, fee_collected::TEXT,
            twitter, discord, tokens_created, tokens_migrated
        FROM account
        WHERE id = $1
        "#,
        user_id.into_inner()
    )
    .fetch_optional(pool.get_ref())
    .await;


    match result {
        Ok(Some(row)) => {


            // let fee_collected = match row.fee_collected {
            //     Some(fee) if fee >= 0 => U256::from(fee as u64),
            //     _ => U256::zero(), // Covers None or negative values
            // };

            let account = Account {
                id: Some(row.id),
                slug: row.slug,
                referral_code: row.referral_code,
                diamond_hand_probability: row.diamond_hand_probability as u32,
                reputation: row.reputation,
                referrer_id: row.referrer_id,
                total_referrals: row.total_referrals.map(|v| v as u32),
                fee_collected: row.fee_collected.map_or(0, |s| s.parse::<u128>().unwrap_or(0)),
                twitter: row.twitter,
                discord: row.discord,
                tokens_created: row.tokens_created.map(|v| v as u32).unwrap_or(0),
                tokens_migrated: row.tokens_migrated.map(|v| v as u32).unwrap_or(0)
            };

            HttpResponse::Ok().json(account)
        }
        Ok(None) => HttpResponse::NotFound().body("Account not found"),
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}


/////////////////////
/// COMMUNITY STUFF ////
/// //////////////////

pub async fn get_diamond_hands(pool: web::Data<PgPool>) -> impl Responder {
    let result = sqlx::query!(
        r#"
        SELECT account_id FROM diamond_hand_list
        "#
    )
    .fetch_all(pool.get_ref()) // Use fetch_all for multiple rows
    .await;

    match result {
        Ok(rows) => {
            let accounts: Vec<String> = rows.into_iter().map(|row| row.account_id).collect();
            HttpResponse::Ok().json(accounts)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}


#[utoipa::path(
    tag = "Communities",
    get,
    path = "/communities",
    responses(
        (status = 200, description = "List of all communities", body = [CommunityResponse]),
        (status = 500, description = "Database error")
    )
)]
#[get("/communities")]
pub async fn get_all_communities(pool: web::Data<PgPool>) -> impl Responder {
    let result = sqlx::query!(
        r#"
        SELECT name, img_url, chain, merkle_root, holder_count, community_score FROM communities
        "#
    )
    .fetch_all(pool.get_ref())
    .await;

    match result {
        Ok(rows) => {
            let communities: Vec<CommunityResponse> = rows
                .into_iter()
                .map(|row| CommunityResponse {
                    name: row.name,
                    img_url: row.img_url,
                    chain: row.chain,
                    merkle_root: hex::encode(row.merkle_root),
                    holder_count: row.holder_count,
                    community_score: row.community_score.map(|bd| bd.to_f32().unwrap_or(0.0)),
                })
                .collect();

            HttpResponse::Ok().json(communities)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}



/// Get Merkle proof for an account and token
#[utoipa::path(
    tag = "Account",
    get,
    path = "/airdrop/proof/{account_id}/{token_id}",
    params(
        ("account_id" = String, Path, description = "Account ID to get the merkle proof for"),
        ("token_id" = String, Path, description = "Token ID to get the merkle proof for")
    ),
    responses(
        (status = 200, description = "Merkle proof retrieved successfully", body = MerkleProofResponse),
        (status = 200, description = "No airdrop found for this account and token"),
        (status = 500, description = "Internal server or database error")
    )
)]
#[get("/airdrop/proof/{account_id}/{token_id}")]
pub async fn get_merkle_proof(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (account_id, token_id) = path.into_inner();
    
    // Get the airdrop details for this account and token
    let result = sqlx::query!(
        r#"
        SELECT 
            ta.token_id,
            ta.merkle_root,
            ta.merkle_proofs,
            ta.community_id,
            ta.community_name,
            ta.total_amount,
            ta.transaction_hash,
            ar.claimed
        FROM token_airdrops ta
        JOIN airdrop_recipients ar ON ta.id = ar.token_airdrop_id
        WHERE ar.account_id = $1 AND ta.token_id = $2
        LIMIT 1
        "#,
        account_id,
        token_id
    )
    .fetch_optional(pool.get_ref())
    .await;
    
    match result {
        Ok(Some(row)) => {
            // Parse the JSONB
            let proofs: serde_json::Value = row.merkle_proofs;
            
            // Extract the proof for this account
            let account_proof = proofs.get(&account_id);
            
            if let Some(proof) = account_proof {
                let response = MerkleProofResponse {
                    token_id: row.token_id,
                    account_id: account_id,
                    merkle_root: hex::encode(row.merkle_root),
                    merkle_proof: proof.clone(),
                    community_id: row.community_id,
                    community_name: row.community_name,
                    total_amount: row.total_amount.to_string(),
                    transaction_hash: row.transaction_hash,
                    claimed: row.claimed,
                };
                
                HttpResponse::Ok().json(response)
            } else {
                HttpResponse::NotFound().json("No merkle proof found for this account")
            }
        },
        Ok(None) => HttpResponse::Ok().json(""),
        Err(e) => {
            eprintln!("Error fetching merkle proof: {:?}", e);
            HttpResponse::InternalServerError().body("Database error")
        }
    }
}

/// Get communities a user is part of
#[utoipa::path(
    tag = "Communities",
    get,
    path = "/account/{account_id}/communities",
    params(
        ("account_id" = String, Path, description = "Account ID to fetch communities for")
    ),
    responses(
        (status = 200, description = "List of communities the user is part of", body = [Community]),
        (status = 500, description = "Internal server or database error")
    )
)]
#[get("/account/{account_id}/communities")]
pub async fn get_account_communities(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> impl Responder {
    let account_id = path.into_inner();
    
    // Direct query to get only the needed fields
    let result = sqlx::query_as!(
        Community,
        r#"
        SELECT 
            c.id,
            c.name,
            c.img_url
        FROM communities c
        JOIN account_communities ac ON c.id = ac.community_id
        WHERE ac.account_id = $1
        "#,
        account_id
    )
    .fetch_all(pool.get_ref())
    .await;
    
    match result {
        Ok(communities) => HttpResponse::Ok().json(communities),
        Err(e) => {
            eprintln!("Error fetching communities: {:?}", e);
            HttpResponse::InternalServerError().body("Database error")
        }
    }
}

/// Get top 100 ranking accounts by reputation
#[utoipa::path(
    tag = "Account",
    get,
    path = "/accounts/ranking",
    responses(
        (status = 200, description = "Top 100 accounts ranked by reputation", body = [Account]),
        (status = 500, description = "Database error")
    )
)]
#[get("/accounts/ranking")]
pub async fn get_top_accounts_ranking(pool: web::Data<PgPool>) -> impl Responder {
    let result = sqlx::query!(
        r#"
        SELECT 
            id, slug, referral_code, diamond_hand_probability, reputation,
            referrer_id, total_referrals, fee_collected::TEXT,
            twitter, discord, tokens_created, tokens_migrated
        FROM account
        WHERE slug IS NULL  
        ORDER BY reputation DESC
        LIMIT 100
        "#
    )
    .fetch_all(pool.get_ref())
    .await;

    match result {
        Ok(rows) => {
            let accounts: Vec<Account> = rows
                .into_iter()
                .map(|row| Account {
                    id: Some(row.id),
                    slug: row.slug,
                    referral_code: row.referral_code,
                    diamond_hand_probability: row.diamond_hand_probability as u32,
                    reputation: row.reputation,
                    referrer_id: row.referrer_id,
                    total_referrals: row.total_referrals.map(|v| v as u32),
                    fee_collected: row.fee_collected.map_or(0, |s| s.parse::<u128>().unwrap_or(0)),
                    twitter: row.twitter,
                    discord: row.discord,
                    tokens_created: row.tokens_created.map(|v| v as u32).unwrap_or(0),
                    tokens_migrated: row.tokens_migrated.map(|v| v as u32).unwrap_or(0)
                })
                .collect();

            HttpResponse::Ok().json(accounts)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AccountRankResponse {
    pub user_id: String,
    pub global_rank: i64,
    pub reputation: Option<i32>,
}

#[utoipa::path(
    tag = "Account",
    get,
    path = "/account/{user_id}/rank",
    params(
        ("user_id" = String, Path, description = "User identifier (address)")
    ),
    responses(
        (status = 200, description = "Returns account's rank and reputation", body = AccountRankResponse),
        (status = 404, description = "Account not found"),
        (status = 500, description = "Database error")
    )
)]
#[get("/account/{user_id}/rank")]
pub async fn get_account_rank(
    pool: web::Data<PgPool>,
    user_id: web::Path<String>,
) -> impl Responder {
    let user_id = user_id.into_inner();
    // Query the materialized view for rank
    let result = sqlx::query!(
        r#"
        SELECT user_id, global_rank, reputation
        FROM account_reputation_rankings
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_optional(pool.get_ref())
    .await;

    match result {
        Ok(Some(row)) => {
            let response = AccountRankResponse {
                user_id: row.user_id.unwrap_or_default(),
                global_rank: row.global_rank.unwrap_or(0),
                reputation: row.reputation,
            };
            HttpResponse::Ok().json(response)
        }
        Ok(None) => HttpResponse::NotFound().body("Account not found in rankings"),
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}

//
// ----- OpenAPI Aggregation -----
//

#[derive(OpenApi)]
#[openapi(
    paths(
        create_account,
        get_account,
        get_top_accounts_ranking,
        get_all_communities,
        get_merkle_proof,
        get_account_communities,
    ),
    components(
        schemas(
            CreateAccountRequest,
            CreateAccountResponse,
            Account,
            AccountData,
            WatchlistActionRequest,
            UpdateAccountRequest, 
            CommunityResponse,
            CultTokensResponse,
            CultTokenDataResponse,
            CultTokenTopHolder,
            TokenTradesResponse,
            MerkleProofResponse,
            Community,
            UpcomingTokensResponse,
            DiamondHandsRequest,
            DiamondHandsResponse
        )
    ),
    tags(
        (name = "Account", description = "Payload for creating, getting account, including optional referral and social handles"),
        (name = "Communities", description = "View all supported communities")
    ),
    servers(
        (url = "/api", description = "API base path")
    )
)]
pub struct ApiDoc;