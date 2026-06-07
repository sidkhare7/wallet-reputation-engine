#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
use actix_web::{
    guard,
    http::header,
    middleware::{Logger, NormalizePath},
    web, App, HttpResponse, HttpServer, Responder,
};
use anyhow::Result;
use dotenv::dotenv;
use futures;
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{collections::HashSet, env, sync::Arc, time::Duration};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
// Async job queue for processing events
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task;

mod auth;
mod community_airdrops;
mod diamond_hands;
mod errors;
mod handlers;
mod models;
mod routes;
pub mod utils;

use crate::utils::misc::to_checksum_address;
use crate::utils::udpate_token_balance::cleanup_duplicate_accounts;
mod webhook_data;
use actix_cors::Cors;
use auth::middleware::ApiGuard;
use community_airdrops::community_airdrops::update_all_communities;
use community_airdrops::compute_all_community_scores::update_all_community_scores;
use diamond_hands::diamond_hands::{
    get_eligible_accounts, select_weighted_sample, update_diamond_hands, SAMPLE_SIZE,
};
use crate::models::{UpcomingTokenPayload, DiamondHandsResponse};
use crate::webhook_data::{
    IncomingCreateTokenData, IncomingSwapData, IncomingTransferData,
    BatchCreateTokenData, BatchSwapData, BatchTransferData,
    BatchWebhookResponse,
};
use csv::Reader;
use std::str::FromStr;

// Event processing queue
struct EventProcessor {
    sender: mpsc::Sender<models::WebhookPayload>,
    pool: Pool<Postgres>,
    event_sender: broadcast::Sender<models::WebhookPayload>,
}

impl EventProcessor {
    async fn start(
        mut receiver: mpsc::Receiver<models::WebhookPayload>,
        pool: Pool<Postgres>,
        max_concurrent: usize,
        event_sender: broadcast::Sender<models::WebhookPayload>,
    ) {
        // Semaphore to limit concurrent processing
        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));

        while let Some(payload) = receiver.recv().await {
            let pool_clone = pool.clone();
            let semaphore_clone = semaphore.clone();

            // Spawn a task with limited concurrency
            task::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();

                // Start a transaction
                let mut tx = match pool_clone.begin().await {
                    Ok(tx) => tx,
                    Err(e) => {
                        log::error!("Failed to start transaction: {}", e);
                        return;
                    }
                };

                let result = match payload.event_type {
                    models::WebhookEventType::CultTokenCreated => {
                        match serde_json::from_value::<handlers::CultTokenCreatedEvent>(
                            payload.data,
                        ) {
                            Ok(event) => handlers::handle_cult_token_created(event, &mut tx).await,
                            Err(_) => return,
                        }
                    }
                    // models::WebhookEventType::CultTokenBuy => {
                    //     match serde_json::from_value::<handlers::CultTokenBuyEvent>(
                    //         payload.data.clone(),
                    //     ) {
                    //         Ok(event) => {
                    //             let result =
                    //                 handlers::handle_cult_token_buy(event.clone(), &mut tx).await;

                    //             // if result.is_ok() {
                    //             //     if let Err(e) = event_sender_clone.send(payload.clone()) {
                    //             //         let error_msg = format!("Failed to broadcast event: {}", e);
                    //             //         log::error!("{}", error_msg);
                    //             //         sentry::capture_message(&error_msg, sentry::Level::Error);
                    //             //     }
                    //             // }

                    //             result
                    //         }
                    //         Err(_) => return,
                    //     }
                    // }
                    // models::WebhookEventType::CultTokenSell => {
                    //     match serde_json::from_value::<CultTokenSellEvent>(payload.data.clone()) {
                    //         Ok(event) => {
                    //             let result =
                    //                 handlers::handle_cult_token_sell(event.clone(), &mut tx).await;

                    //             // if result.is_ok() {
                    //             //     if let Err(e) = event_sender_clone.send(payload.clone()) {
                    //             //         let error_msg = format!("Failed to broadcast event: {}", e);
                    //             //         log::error!("{}", error_msg);
                    //             //         sentry::capture_message(&error_msg, sentry::Level::Error);
                    //             //     }
                    //             // }

                    //             result
                    //         }
                    //         Err(_) => return,
                    //     }
                    // }
                    // models::WebhookEventType::TokenClaimed => {
                    //     match serde_json::from_value::<handlers::TokensClaimedEvent>(payload.data) {
                    //         Ok(event) => handlers::handle_token_claimed(event, &mut tx).await,
                    //         Err(_) => return,
                    //     }
                    // }
                    // models::WebhookEventType::CultMarketGraduated => {
                    //     match serde_json::from_value::<handlers::CultMarketGraduatedEvent>(
                    //         payload.data,
                    //     ) {
                    //         Ok(event) => {
                    //             handlers::handle_cult_market_graduated(event, &mut tx).await
                    //         }
                    //         Err(_) => return,
                    //     }
                    // }
                    models::WebhookEventType::CultTokenTransfer => {
                        match serde_json::from_value::<handlers::CultTokenTransferEvent>(
                            payload.data,
                        ) {
                            Ok(event) => handlers::handle_cult_token_transfer(event, &mut tx).await,
                            Err(_) => return,
                        }
                    }
                    models::WebhookEventType::CultSwap => {
                        match serde_json::from_value::<handlers::CultSwapEvent>(payload.data) {
                            Ok(event) => handlers::handle_cult_swap(event, &mut tx).await,
                            Err(_) => return,
                        }
                    }
                };

                // Commit or rollback the transaction
                match result {
                    Ok(_) => {
                        // If result is Ok, attempt to commit
                        if let Err(e) = tx.commit().await {
                            let error_msg = format!(
                                "Failed to commit transaction after successful processing: {}",
                                e
                            );
                            log::error!("{}", error_msg);
                            sentry::capture_message(&error_msg, sentry::Level::Error);
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Event processing failed: {}. Rolling back.", e);
                        log::error!("{}", error_msg);
                        sentry::capture_message(&error_msg, sentry::Level::Error);

                        if let Err(rollback_err) = tx.rollback().await {
                            let rollback_error_msg =
                                format!("Failed to rollback transaction: {}", rollback_err);
                            log::error!("{}", rollback_error_msg);
                            sentry::capture_message(&rollback_error_msg, sentry::Level::Error);
                        }
                    }
                }
            });
        }
    }
}

async fn create_token_webhook_handler(
    payload: web::Json<BatchCreateTokenData>,
    event_processor: web::Data<mpsc::Sender<models::WebhookPayload>>,
) -> impl Responder {
    log::info!("Processing create token webhook request");

    let batch_data = payload.into_inner().to_vec();
    let total_count = batch_data.len();
    let mut processed = 0;
    let mut failed = 0;
    let mut first_event_id = String::new();
    let mut single_item_success = false;

    log::info!("Processing {} create token events", total_count);

    for incoming_data in batch_data {
        let event_id = incoming_data.transaction_hash.clone();
        
        // Store first event ID for single-item response
        if first_event_id.is_empty() {
            first_event_id = event_id.clone();
        }

        match process_single_create_token(incoming_data, &event_processor).await {
            Ok(_) => {
                processed += 1;
                if total_count == 1 {
                    single_item_success = true;
                }
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    let response_status = if failed == 0 {
        "success"
    } else if processed == 0 {
        "error"
    } else {
        "partial_success"
    };

    // Return single response format for backward compatibility if only one item
    if total_count == 1 {
        if single_item_success {
            return HttpResponse::Ok().json(models::WebhookResponse {
                status: "success".to_string(),
                event_id: first_event_id,
            });
        } else {
            return HttpResponse::BadRequest().json(models::WebhookResponse {
                status: "error".to_string(),
                event_id: first_event_id,
            });
        }
    }

    // Return batch response format for multiple items
    let response = BatchWebhookResponse {
        status: response_status.to_string(),
        processed,
        failed,
    };

    if failed == 0 {
        HttpResponse::Ok().json(response)
    } else if processed > 0 {
        HttpResponse::MultiStatus().json(response)
    } else {
        HttpResponse::BadRequest().json(response)
    }
}

async fn process_single_create_token(
    incoming_data: IncomingCreateTokenData,
    event_processor: &mpsc::Sender<models::WebhookPayload>,
) -> Result<(), anyhow::Error> {
    let event_id = incoming_data.transaction_hash.clone();

    // Convert to CultTokenCreatedEvent
    let cult_token_created_event = incoming_data.to_cult_token_created_event()
        .map_err(|e| anyhow::anyhow!("Failed to convert webhook data: {}", e))?;

    // Serialize the event data
    let event_data = serde_json::to_value(&cult_token_created_event)
        .map_err(|e| anyhow::anyhow!("Failed to serialize event data: {}", e))?;

    let webhook_payload = models::WebhookPayload {
        id: event_id.clone(),
        event_type: models::WebhookEventType::CultTokenCreated,
        data: event_data,
        timestamp: chrono::Utc::now().timestamp(),
    };

    // Use try_send for non-blocking operation
    match event_processor.try_send(webhook_payload) {
        Ok(_) => Ok(()),
        Err(mpsc::error::TrySendError::Full(_)) => {
            let error_msg = format!("Event queue is full for event {}", event_id);
            log::warn!("{}", error_msg);
            sentry::capture_message(&error_msg, sentry::Level::Warning);
            Err(anyhow::anyhow!("Event queue is full"))
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            let error_msg = format!("Event processor is closed for event {}", event_id);
            log::error!("{}", error_msg);
            sentry::capture_message(&error_msg, sentry::Level::Error);
            Err(anyhow::anyhow!("Event processor is closed"))
        }
    }
}


async fn transfer_token_webhook_handler(
    payload: web::Json<BatchTransferData>,
    event_processor: web::Data<mpsc::Sender<models::WebhookPayload>>,
) -> impl Responder {
    log::info!("Processing token transfer webhook request");

    let batch_data = payload.into_inner().to_vec();
    let total_count = batch_data.len();
    let mut processed = 0;
    let mut failed = 0;
    let mut first_event_id = String::new();
    let mut single_item_success = false;

    log::info!("Processing {} transfer token events", total_count);

    for incoming_data in batch_data {
        let event_id = incoming_data.transaction_hash.clone();
        
        // Store first event ID for single-item response
        if first_event_id.is_empty() {
            first_event_id = event_id.clone();
        }

        match process_single_transfer_token(incoming_data, &event_processor).await {
            Ok(_) => {
                processed += 1;
                if total_count == 1 {
                    single_item_success = true;
                }
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    let response_status = if failed == 0 {
        "success"
    } else if processed == 0 {
        "error"
    } else {
        "partial_success"
    };

    // Return single response format for backward compatibility if only one item
    if total_count == 1 {
        if single_item_success {
            return HttpResponse::Ok().json(models::WebhookResponse {
                status: "success".to_string(),
                event_id: first_event_id,
            });
        } else {
            return HttpResponse::BadRequest().json(models::WebhookResponse {
                status: "error".to_string(),
                event_id: first_event_id,
            });
        }
    }

    // Return batch response format for multiple items
    let response = BatchWebhookResponse {
        status: response_status.to_string(),
        processed,
        failed,
    };

    if failed == 0 {
        HttpResponse::Ok().json(response)
    } else if processed > 0 {
        HttpResponse::MultiStatus().json(response)
    } else {
        HttpResponse::BadRequest().json(response)
    }
}

async fn process_single_transfer_token(
    incoming_data: IncomingTransferData,
    event_processor: &mpsc::Sender<models::WebhookPayload>,
) -> Result<(), anyhow::Error> {
    let event_id = incoming_data.transaction_hash.clone();

    let cult_token_transfer_event = incoming_data.to_cult_token_transfer_event()
        .map_err(|e| anyhow::anyhow!("Failed to convert webhook data: {}", e))?;

    let event_data = serde_json::to_value(&cult_token_transfer_event)
        .map_err(|e| anyhow::anyhow!("Failed to serialize event data: {}", e))?;

    let webhook_payload = models::WebhookPayload {
        id: event_id.clone(),
        event_type: models::WebhookEventType::CultTokenTransfer,
        data: event_data,
        timestamp: chrono::Utc::now().timestamp(),
    };

    match event_processor.try_send(webhook_payload) {
        Ok(_) => Ok(()),
        Err(mpsc::error::TrySendError::Full(_)) => {
            let error_msg = format!("Event queue is full for event {}", event_id);
            log::warn!("{}", error_msg);
            sentry::capture_message(&error_msg, sentry::Level::Warning);
            Err(anyhow::anyhow!("Event queue is full"))
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            let error_msg = format!("Event processor is closed for event {}", event_id);
            log::error!("{}", error_msg);
            sentry::capture_message(&error_msg, sentry::Level::Error);
            Err(anyhow::anyhow!("Event processor is closed"))
        }
    }
}

async fn swap_webhook_handler(
    payload: web::Json<BatchSwapData>,
    event_processor: web::Data<mpsc::Sender<models::WebhookPayload>>,
) -> impl Responder {
    log::info!("Processing swap webhook request");

    let batch_data = payload.into_inner().to_vec();
    let total_count = batch_data.len();
    let mut processed = 0;
    let mut failed = 0;
    let mut first_event_id = String::new();
    let mut single_item_success = false;

    log::info!("Processing {} swap events", total_count);

    for incoming_data in batch_data {
        let event_id = incoming_data.transaction_hash.clone();
        
        // Store first event ID for single-item response
        if first_event_id.is_empty() {
            first_event_id = event_id.clone();
        }

        match process_single_swap(incoming_data, &event_processor).await {
            Ok(_) => {
                processed += 1;
                if total_count == 1 {
                    single_item_success = true;
                }
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    let response_status = if failed == 0 {
        "success"
    } else if processed == 0 {
        "error"
    } else {
        "partial_success"
    };

    // Return single response format for backward compatibility if only one item
    if total_count == 1 {
        if single_item_success {
            return HttpResponse::Ok().json(models::WebhookResponse {
                status: "success".to_string(),
                event_id: first_event_id,
            });
        } else {
            return HttpResponse::BadRequest().json(models::WebhookResponse {
                status: "error".to_string(),
                event_id: first_event_id,
            });
        }
    }

    // Return batch response format for multiple items
    let response = BatchWebhookResponse {
        status: response_status.to_string(),
        processed,
        failed,
    };

    if failed == 0 {
        HttpResponse::Ok().json(response)
    } else if processed > 0 {
        HttpResponse::MultiStatus().json(response)
    } else {
        HttpResponse::BadRequest().json(response)
    }
}

async fn process_single_swap(
    incoming_data: IncomingSwapData,
    event_processor: &mpsc::Sender<models::WebhookPayload>,
) -> Result<(), anyhow::Error> {
    let event_id = incoming_data.transaction_hash.clone();

    let cult_swap_event = incoming_data.to_cult_swap_event()
        .map_err(|e| anyhow::anyhow!("Failed to convert webhook data: {}", e))?;

    let event_data = serde_json::to_value(&cult_swap_event)
        .map_err(|e| anyhow::anyhow!("Failed to serialize event data: {}", e))?;

    let webhook_payload = models::WebhookPayload {
        id: event_id.clone(),
        event_type: models::WebhookEventType::CultSwap,
        data: event_data,
        timestamp: chrono::Utc::now().timestamp(),
    };

    match event_processor.try_send(webhook_payload) {
        Ok(_) => Ok(()),
        Err(mpsc::error::TrySendError::Full(_)) => {
            let error_msg = format!("Event queue is full for event {}", event_id);
            log::warn!("{}", error_msg);
            sentry::capture_message(&error_msg, sentry::Level::Warning);
            Err(anyhow::anyhow!("Event queue is full"))
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            let error_msg = format!("Event processor is closed for event {}", event_id);
            log::error!("{}", error_msg);
            sentry::capture_message(&error_msg, sentry::Level::Error);
            Err(anyhow::anyhow!("Event processor is closed"))
        }
    }
}

async fn community_airdrops_handler(
    pool: web::Data<sqlx::PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let result: Result<_, Box<dyn std::error::Error>> = async {
        let api_key = env::var("ALCHEMY_API_KEY")
            .map_err(|_| "Failed to retrieve Alchemy API key from environment")?;

        if api_key.trim().is_empty() {
            return Err("Alchemy API key is empty".into());
        }

        // let update_result = update_all_communities(&pool, &api_key)
        //     .await
        //     .map_err(|e| format!("Community update failed: {}", e))?;

        Ok(())
    }
    .await;

    // Comprehensive error handling and response generation
    match result {
        Ok(updated_count) => Ok(HttpResponse::Ok().json(json!({
            "message": "Community List Updated Successfully",
            "communities_updated": updated_count
        }))),
        Err(e) => {
            // Log the error for internal tracking
            log::error!("Community airdrops handler error: {}", e);

            // Differentiate error responses
            if e.to_string().contains("API key") {
                Ok(HttpResponse::Unauthorized().body("Authentication failed: Invalid API key"))
            } else if e.to_string().contains("network") {
                Ok(HttpResponse::ServiceUnavailable().body("Network connectivity issue"))
            } else if e.to_string().contains("database") {
                Ok(HttpResponse::InternalServerError().body("Database operation failed"))
            } else {
                Ok(HttpResponse::InternalServerError().body(format!("Execution failed: {}", e)))
            }
        }
    }
}

// Helper function to parse CSV and extract addresses
fn parse_csv_addresses(csv_content: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut reader = Reader::from_reader(csv_content.as_bytes());
    let mut addresses = Vec::new();
    
    // Try to find "address" column
    let headers = reader.headers()?.clone();
    let address_index = headers.iter()
        .position(|h| h.to_lowercase() == "address")
        .ok_or("No 'address' column found in CSV")?;
    
    for result in reader.records() {
        let record = result?;
        if let Some(address) = record.get(address_index) {
            let trimmed_address = address.trim().to_string();
            if !trimmed_address.is_empty() {
                addresses.push(trimmed_address);
            }
        }
    }
    
    Ok(addresses)
}

// Helper function to parse CSV and extract balance update data
fn parse_csv_balance_data(csv_content: &str) -> Result<Vec<(String, String, String, String)>, Box<dyn std::error::Error>> {
    let mut reader = Reader::from_reader(csv_content.as_bytes());
    let mut balance_data = Vec::new();
    
    // Find required columns
    let headers = reader.headers()?.clone();
    let account_index = headers.iter()
        .position(|h| h.to_lowercase() == "account_id")
        .ok_or("No 'account_id' column found in CSV")?;
    let token_index = headers.iter()
        .position(|h| h.to_lowercase() == "token_id")
        .ok_or("No 'token_id' column found in CSV")?;
    let contract_balance_index = headers.iter()
        .position(|h| h.to_lowercase() == "contract_balance")
        .ok_or("No 'contract_balance' column found in CSV")?;
    let balance_difference_index = headers.iter()
        .position(|h| h.to_lowercase() == "balance_difference")
        .ok_or("No 'balance_difference' column found in CSV")?;
    
    for result in reader.records() {
        let record = result?;
        if let (Some(account_id), Some(token_id), Some(contract_balance), Some(balance_difference)) = (
            record.get(account_index),
            record.get(token_index),
            record.get(contract_balance_index),
            record.get(balance_difference_index)
        ) {
            let trimmed_account = account_id.trim().to_string();
            let trimmed_token = token_id.trim().to_string();
            let trimmed_balance = contract_balance.trim().to_string();
            let trimmed_difference = balance_difference.trim().to_string();
            
            if !trimmed_account.is_empty() && !trimmed_token.is_empty() && !trimmed_balance.is_empty() {
                balance_data.push((trimmed_account, trimmed_token, trimmed_balance, trimmed_difference));
            }
        }
    }
    
    Ok(balance_data)
}

async fn custom_diamond_hands_csv_handler(
    pool: web::Data<sqlx::PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    log::info!("Processing diamond hands with CSV file - reading from local file");
    
    // Read the CSV file from the local filesystem
    let csv_content = match std::fs::read_to_string("output.csv") {
        Ok(content) => content,
        Err(e) => {
            let error_msg = format!("Failed to read CSV file: {}", e);
            log::error!("{}", error_msg);
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to read CSV file",
                "message": error_msg
            })));
        }
    };
    

    
    match parse_csv_addresses(&csv_content) {
        Ok(addresses) => {
            log::info!("Parsed {} addresses from local CSV file", addresses.len());
            
            let original_count = addresses.len();
            
            // Convert all addresses to checksum format
            let mut checksum_addresses = Vec::new();
            for address in addresses {
                match to_checksum_address(&address) {
                    Ok(checksum_addr) => checksum_addresses.push(checksum_addr),
                    Err(e) => {
                        let error_msg = format!("Invalid address '{}': {}", address, e);
                        log::error!("{}", error_msg);
                        return Ok(HttpResponse::BadRequest().json(json!({
                            "error": "Invalid address format",
                            "message": error_msg
                        })));
                    }
                }
            }
            
            log::info!("Converted {} addresses to checksum format", checksum_addresses.len());
            
            // Remove duplicates by converting to HashSet and back to Vec
            let unique_addresses: std::collections::HashSet<String> = checksum_addresses.into_iter().collect();
            let deduplicated_addresses: Vec<String> = unique_addresses.into_iter().collect();
            
            let deduplicated_count = deduplicated_addresses.len();
            let duplicates_removed = original_count - deduplicated_count;
            
            log::info!("Removed {} duplicates. Final count: {} unique addresses", duplicates_removed, deduplicated_count);
            
            // Save unique addresses to file
            let output_file = "unique_checksum_addresses.txt";
            let addresses_text = deduplicated_addresses.join("\n");
            match std::fs::write(output_file, addresses_text) {
                Ok(_) => log::info!("Saved {} unique addresses to {}", deduplicated_count, output_file),
                Err(e) => {
                    let error_msg = format!("Failed to save addresses to file: {}", e);
                    log::error!("{}", error_msg);
                    return Ok(HttpResponse::InternalServerError().json(json!({
                        "error": "Failed to save addresses",
                        "message": error_msg
                    })));
                }
            }
            process_diamond_hands_addresses(pool, deduplicated_addresses).await;
            // Return the list for verification instead of processing immediately
            Ok(HttpResponse::Ok().json(json!({
                "message": "Address list processed and deduplicated",
                "original_count": original_count,
                "deduplicated_count": deduplicated_count,
                "duplicates_removed": duplicates_removed,
                "saved_to_file": output_file,
                "note": "Unique addresses saved to file. Verify the list and call the processing endpoint to proceed."
            })))
        },
        Err(e) => {
            let error_msg = format!("CSV parsing error: {}", e);
            log::error!("{}", error_msg);
            Ok(HttpResponse::BadRequest().json(json!({
                "error": "Invalid CSV format",
                "message": error_msg
            })))
        }
    }
}

async fn custom_diamond_hands_handler(
    pool: web::Data<sqlx::PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    // Fallback handler with hardcoded addresses for backwards compatibility
    let addresses = vec![
        "0x60187Bc4949eE2F01b507a9F77ad615093f44260".to_string(),
        "0x74d8dF0c2b9EDdFB23a15099BbcD0a35b80BaF90".to_string(),
        "0x7909bC836c98bE432c43CF58CE9442a6564026aE".to_string(),
        "0x2dC727b15203992B65D7ADbc0108781f1Cb1F9F3".to_string(),
        "0x27fAa6497818EC151fb1828D68b60fB6966e4063".to_string(),
        "0xcfa038455b54714821f291814071161c9870B891".to_string(),
        "0x57aECBA033b104Ec1abDE9c3ff6b1430A16a69b8".to_string(),
        "0xa85367272da042052e05459Ca3390c185DDb55ca".to_string(),
        "0x3e8B5487Ab1389394d0F19Cdc050Ee1c741e4B30".to_string(),
        "0x3e53f3472967BeBf147074de5D7F15bb6412D049".to_string()
    ];
    log::info!("Using default hardcoded addresses");
    process_diamond_hands_addresses(pool, addresses).await
}

// Common function to process diamond hands for a list of addresses
async fn process_diamond_hands_addresses(
    pool: web::Data<sqlx::PgPool>,
    addresses: Vec<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let result: Result<DiamondHandsResponse, Box<dyn std::error::Error>> = async {
        if addresses.is_empty() {
            return Err("No addresses provided".into());
        }
        
        let mut tx = pool.begin().await?;
        let mut accounts_created = 0;
        let mut created_addresses = Vec::new();
        
        // Limit concurrent contract calls to avoid rate limiting
        let semaphore = Arc::new(tokio::sync::Semaphore::new(10)); // Limit to 10 concurrent calls
        
        // Create accounts for all addresses in parallel with rate limiting
        let account_creation_futures: Vec<_> = addresses
            .iter()
            .map(|address| {
                let pool_clone = pool.get_ref().clone();
                let semaphore_clone = semaphore.clone();
                async move {
                    let _permit = semaphore_clone.acquire().await?;
                    let mut tx = pool_clone.begin().await?;
                    let result = handlers::create_account(&mut tx, address, None, None).await;
                    if result.is_ok() {
                        tx.commit().await?;
                    } else {
                        tx.rollback().await?;
                    }
                    Ok::<_, Box<dyn std::error::Error>>((address.clone(), result))
                }
            })
            .collect();
        
        // Wait for all account creations to complete
        let results = futures::future::join_all(account_creation_futures).await;
        
        // Process results and collect created addresses
        for result in results {
            match result {
                Ok((address, Ok(created))) => {
                    if created {
                        accounts_created += 1;
                        created_addresses.push(address);
                    }
                }
                Ok((address, Err(e))) => {
                    log::warn!("Failed to create account for address {}: {}", address, e);
                    // Continue processing other addresses
                }
                Err(e) => {
                    log::warn!("Failed to process account creation: {}", e);
                    // Continue processing other addresses
                }
            }
        }
        
        // Update diamond hands only for accounts that were created
        if !created_addresses.is_empty() {
            if created_addresses.len() > 5000 {
                // Use high-performance version for very large datasets
                diamond_hands::diamond_hands::update_diamond_hands_high_performance(&mut tx, created_addresses).await?;
            } else {
                // Use standard optimized version for smaller datasets
                update_diamond_hands(&mut tx, created_addresses).await?;
            }
        }
        tx.commit().await?;
        
        Ok(DiamondHandsResponse {
            message: "Diamond hands processing completed successfully".to_string(),
            accounts_processed: addresses.len(),
            accounts_created,
            diamond_hands_updated: true,
        })
    }
    .await;

    // Match and handle different error scenarios
    match result {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => {
            log::error!("diamond_hands processing failed: {}", e);
            let error_msg = e.to_string();
            if error_msg.contains("database connection") {
                Ok(HttpResponse::ServiceUnavailable().json(json!({
                    "error": "Database connection error",
                    "message": error_msg
                })))
            } else if error_msg.contains("No eligible accounts") {
                Ok(HttpResponse::BadRequest().json(json!({
                    "error": "No eligible accounts found",
                    "message": error_msg
                })))
            } else {
                Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Execution failed",
                    "message": error_msg
                })))
            }
        }
    }
}

async fn diamond_hands_handler(
    pool: web::Data<sqlx::PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let result: Result<_, Box<dyn std::error::Error>> = async {
        let mut tx = pool.begin().await?;
        let accounts = get_eligible_accounts(&mut tx).await?;
        let selected = select_weighted_sample(accounts, SAMPLE_SIZE);
        update_diamond_hands(&mut tx, selected).await?;
        tx.commit().await?;
        Ok(())
    }
    .await;

    // Match and handle different error scenarios
    match result {
        Ok(_) => Ok(HttpResponse::Ok().body("Diamond hands executed successfully")),
        Err(e) => {
            log::error!("Diamond hands execution failed: {}", e);
            if e.to_string().contains("database connection") {
                Ok(HttpResponse::ServiceUnavailable().body("Database connection error"))
            } else if e.to_string().contains("No eligible accounts") {
                Ok(HttpResponse::BadRequest().body("No eligible accounts found"))
            } else {
                Ok(HttpResponse::InternalServerError().body(format!("Execution failed: {}", e)))
            }
        }
    }
}

async fn run_token_stats_handler(
    pool: web::Data<sqlx::PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let start_time = std::time::Instant::now();
    
    // Use sequential optimized processing to maintain data dependencies
    let (z_score_records_updated, users_updated) = match utils::update_token_stats::sequential_update_all_stats_optimized(
        pool.get_ref(), 
        Some(100),  // token limit
        Some(100000)  // user limit
    ).await {
        Ok((z_scores, users)) => {
            log::info!("Sequential optimized update completed - Z-scores: {}, Users: {}", z_scores, users);
            (z_scores, users)
        }
        Err(e) => {
            log::error!("Failed to execute sequential optimized update: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to update token stats and user reputation",
                "message": e.to_string()
            })));
        }
    };

    // Refresh the materialized view after updating stats
    match sqlx::query("REFRESH MATERIALIZED VIEW account_reputation_rankings")
        .execute(pool.get_ref())
        .await
    {
        Ok(_) => {
            log::info!("Successfully refreshed account_reputation_rankings materialized view");
        }
        Err(e) => {
            log::error!("Failed to refresh account_reputation_rankings materialized view: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to refresh materialized view",
                "message": e.to_string()
            })));
        }
    }

    let duration = start_time.elapsed();

    
    log::info!("Token stats update completed in {:?}", duration);
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Token stats and user reputation updated successfully (sequential optimized)",
        "z_score_records_updated": z_score_records_updated,
        "users_updated": users_updated,
        "execution_time_ms": duration.as_millis()
    })))
}

async fn add_upcoming_token_handler(
    pool: web::Data<sqlx::PgPool>,
    payload: web::Json<UpcomingTokenPayload>,
) -> Result<HttpResponse, actix_web::Error> {
    let token_data = payload.into_inner();

    log::info!("Attempting to add upcoming token: {}", token_data.id);

    let result = sqlx::query(
        r#"
        INSERT INTO upcoming_tokens (
            id, token_creator, token_uri, name, symbol, release_date, 
            total_airdrop_recipient_count, ipfs_content, community_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(&token_data.id)
    .bind(&token_data.token_creator)
    .bind(&token_data.token_uri)
    .bind(&token_data.name)
    .bind(&token_data.symbol)
    .bind(token_data.release_date) // DateTime<Utc>
    .bind(token_data.total_airdrop_recipient_count) // Option<i64>
    .bind(&token_data.ipfs_content)
    .bind(token_data.community_id) // Option<String>
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(exec_result) => {
            if exec_result.rows_affected() == 1 {
                log::info!("Successfully added upcoming token: {}", token_data.id);
                Ok(HttpResponse::Created().json(json!({
                    "status": "success",
                    "message": "Upcoming token added successfully",
                    "token_id": token_data.id
                })))
            } else {
                log::error!(
                    "Failed to add upcoming token (no rows affected): {}",
                    token_data.id
                );
                Ok(HttpResponse::InternalServerError().json(json!({
                    "status": "error",
                    "message": "Failed to add upcoming token, no rows affected.",
                    "token_id": token_data.id
                })))
            }
        }
        Err(e) => {
            log::error!("Failed to add upcoming token {}: {}", token_data.id, e);
            if let Some(db_err) = e.as_database_error() {
                if db_err.is_unique_violation() {
                    return Ok(HttpResponse::Conflict().json(json!({
                        "status": "error",
                        "message": format!("Token with ID {} already exists.", token_data.id),
                        "token_id": token_data.id
                    })));
                }
            }
            Ok(HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "message": "Failed to add upcoming token due to a server error.",
                "token_id": token_data.id
            })))
        }
    }
}

async fn update_token_balance_handler(
    pool: web::Data<sqlx::PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    match utils::udpate_token_balance::update_all_token_holdings_from_contracts(pool.get_ref()).await {
        Ok(results) => {
            let total_accounts: usize = results.iter().map(|r| r.total_accounts).sum();
            let successful_updates: usize = results.iter().map(|r| r.successful_updates).sum();
            let failed_updates: usize = results.iter().map(|r| r.failed_updates).sum();
            
            Ok(HttpResponse::Ok().json(json!({
                "status": "success",
                "message": "Token balance update completed",
                "tokens_processed": results.len(),
                "total_accounts": total_accounts,
                "successful_updates": successful_updates,
                "failed_updates": failed_updates,
            })))
        }
        Err(e) => {
            log::error!("Failed to update token balances: {}", e);
            Ok(HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "message": format!("Failed to update token balances: {}", e)
            })))
        }
    }
}

async fn update_token_balance_from_csv_handler(
    pool: web::Data<sqlx::PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    log::info!("Processing token balance update from CSV file");
    
    // Read CSV file
    let csv_content = match std::fs::read_to_string("gmonad_token_balance.csv") {
        Ok(content) => content,
        Err(e) => {
            log::error!("Failed to read CSV file: {}", e);
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to read CSV file",
                "message": e.to_string()
            })));
        }
    };

    // Parse CSV data
    let balance_data = match parse_csv_balance_data(&csv_content) {
        Ok(data) => data,
        Err(e) => {
            log::error!("CSV parsing error: {}", e);
            return Ok(HttpResponse::BadRequest().json(json!({
                "error": "Invalid CSV format", 
                "message": e.to_string()
            })));
        }
    };

    log::info!("Parsed {} balance records from CSV", balance_data.len());
    
    let mut successful_updates = 0;
    let mut failed_updates = 0;

    // First, collect all accounts that need to be created and all valid updates
    let mut all_accounts = std::collections::HashSet::new();
    let mut all_updates = Vec::new();
    
    // Process all data to determine what needs updating
    for (account_id, token_id, contract_balance, balance_difference) in balance_data.iter() {
        match sqlx::types::BigDecimal::from_str(contract_balance) {
            Ok(balance_bd) => {
                // Skip accounts with zero balance
                if balance_bd > sqlx::types::BigDecimal::from(0) {
                    // Parse the balance difference from CSV
                    match sqlx::types::BigDecimal::from_str(balance_difference) {
                        Ok(difference) => {
                            let abs_difference = difference.abs();
                            let threshold = sqlx::types::BigDecimal::from(1_000_000_000i64);
                            
                            // Only update if difference is greater than 1 billion
                            if abs_difference > threshold {
                                all_accounts.insert(account_id.clone());
                                all_updates.push((account_id.clone(), token_id.clone(), balance_bd));
                                log::info!("Queuing update for account {} - difference: {}", account_id, difference);
                            } else {
                                log::debug!("Skipping account {} - difference {} is below threshold {}", 
                                    account_id, difference, threshold);
                            }
                        }
                        Err(_) => {
                            log::warn!("Invalid difference format for account {}: {}", account_id, balance_difference);
                        }
                    }
                } else {
                    log::debug!("Skipping account {} with zero balance", account_id);
                }
            }
            Err(_) => {
                failed_updates += 1;
                log::warn!("Invalid balance format for account {}: {}", account_id, contract_balance);
            }
        }
    }

    log::info!("Found {} accounts to create and {} balance updates to process", all_accounts.len(), all_updates.len());

    if all_updates.is_empty() {
        log::info!("No updates needed - all differences are below threshold");
        return Ok(HttpResponse::Ok().json(json!({
            "message": "Token balance update completed - no updates needed",
            "total_records": balance_data.len(),
            "successful_updates": 0,
            "failed_updates": failed_updates
        })));
    }

    // Bulk create all accounts first
    if !all_accounts.is_empty() {
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                log::error!("Failed to start transaction for account creation: {}", e);
                return Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to create accounts",
                    "message": e.to_string()
                })));
            }
        };

        // Build bulk account insert query
        let mut account_query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO account (id, diamond_hand_probability) VALUES "
        );

        let mut first_account = true;
        for account_id in all_accounts.iter() {
            if !first_account {
                account_query_builder.push(", ");
            }
            first_account = false;
            
            account_query_builder.push("(")
                .push_bind(account_id)
                .push(", 0)");
        }
        
        account_query_builder.push(" ON CONFLICT (id) DO NOTHING");

        match account_query_builder.build().execute(&mut *tx).await {
            Ok(_) => {
                match tx.commit().await {
                    Ok(_) => {
                        log::info!("Successfully created {} accounts in bulk", all_accounts.len());
                    }
                    Err(e) => {
                        log::error!("Failed to commit account creation: {}", e);
                        return Ok(HttpResponse::InternalServerError().json(json!({
                            "error": "Failed to commit account creation",
                            "message": e.to_string()
                        })));
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to execute bulk account creation: {}", e);
                let _ = tx.rollback().await;
                return Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to create accounts",
                    "message": e.to_string()
                })));
            }
        }
    }

    // Process token balance updates in larger batches (data is already filtered)
    for batch in all_updates.chunks(1000) {
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                log::error!("Failed to start transaction: {}", e);
                failed_updates += batch.len();
                continue;
            }
        };

        // Build bulk upsert query for token balances
        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO token_balance (account_id, token_id, holdings_value, first_bought, volume, buy_volume, sell_volume, holding_duration, pnl, cost_basis, duration_z, pnl_z, value_z, volume_z, buy_volume_z, sell_volume_z, last_updated) VALUES "
        );

        let mut valid_records = 0;
        for (account_id, token_id, balance_bd) in batch.iter() {
            if valid_records > 0 {
                query_builder.push(", ");
            }
            query_builder.push("(")
                .push_bind(account_id)
                .push(", ")
                .push_bind(token_id)
                .push(", ")
                .push_bind(balance_bd)
                .push(", COALESCE((SELECT first_bought FROM token_balance WHERE account_id = ")
                .push_bind(account_id)
                .push(" AND token_id = ")
                .push_bind(token_id)
                .push("), NOW()), 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, NOW())");
            valid_records += 1;
        }

        query_builder.push(
            " ON CONFLICT (account_id, token_id) 
             DO UPDATE SET 
                holdings_value = EXCLUDED.holdings_value,
                last_updated = NOW()"
        );

        match query_builder.build().execute(&mut *tx).await {
            Ok(result) => {
                match tx.commit().await {
                    Ok(_) => {
                        let rows_updated = result.rows_affected() as usize;
                        successful_updates += rows_updated;
                        log::info!("Batch completed: {} records updated", rows_updated);
                    }
                    Err(e) => {
                        log::error!("Failed to commit batch: {}", e);
                        failed_updates += valid_records;
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to execute batch query: {}", e);
                let _ = tx.rollback().await;
                failed_updates += valid_records;
            }
        }
    }

    let response = json!({
        "message": "Token balance update completed",
        "total_records": balance_data.len(),
        "successful_updates": successful_updates,
        "failed_updates": failed_updates
    });

    if failed_updates == 0 {
        Ok(HttpResponse::Ok().json(response))
    } else if successful_updates > 0 {
        Ok(HttpResponse::PartialContent().json(response))
    } else {
        Ok(HttpResponse::InternalServerError().json(response))
    }
}

async fn cleanup_duplicate_accounts_handler(
    pool: web::Data<sqlx::PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    log::info!("Starting cleanup of duplicate accounts");
    
    match cleanup_duplicate_accounts(pool.get_ref()).await {
        Ok(result) => {
            log::info!("Duplicate account cleanup completed successfully");
            
            let has_errors = !result.errors.is_empty();
            let errors = if has_errors { Some(result.errors) } else { None };
            
            let response = json!({
                "status": "success",
                "message": "Duplicate account cleanup completed",
                "duplicate_groups": result.duplicate_groups,
                "accounts_deleted": result.accounts_deleted,
                "accounts_preserved": result.accounts_preserved,
                "errors": errors
            });
            
            if has_errors {
                Ok(HttpResponse::PartialContent().json(response))
            } else {
                Ok(HttpResponse::Ok().json(response))
            }
        }
        Err(e) => {
            log::error!("Failed to cleanup duplicate accounts: {}", e);
            Ok(HttpResponse::InternalServerError().json(json!({
                "status": "error",
                "message": format!("Failed to cleanup duplicate accounts: {}", e)
            })))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables
    dotenv().ok();

    //panic!("Everything is on fire!");
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .init();

    println!("Started Server successfully");

    // Configuration from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    
    // Server address configuration - works with both local and Render deployment
    let server_address = if let Ok(port) = env::var("PORT") {
        // Render provides PORT environment variable
        format!("0.0.0.0:{}", port)
    } else {
        // Use SERVER_ADDRESS if set (for local development), otherwise default
        env::var("SERVER_ADDRESS").unwrap_or_else(|_| "127.0.0.1:9000".to_string())
    };
    
    let webhook_secret = env::var("WEBHOOK_SECRET").expect("WEBHOOK_SECRET must be set");

    // Database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(50)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Initialize with hardcoded token contracts
    let contracts_count = utils::misc::get_registered_tokens_count();
    log::info!("Initialized with {} hardcoded token contracts", contracts_count);
    println!("Initialized with {} hardcoded token contracts", contracts_count);
    // Webhook configuration
    let webhook_config = models::WebhookConfig {
        secret_key: webhook_secret,
        max_concurrent_jobs: 100,
        job_queue_buffer: 10_000,
    };

    let (event_sender, _) = broadcast::channel(100);
    let event_sender_clone = event_sender.clone();

    // Create event processing channel
    let (sender, receiver) = mpsc::channel(webhook_config.job_queue_buffer);

    // Start background event processor
    tokio::spawn(EventProcessor::start(
        receiver,
        pool.clone(),
        webhook_config.max_concurrent_jobs,
        event_sender.clone(),
    ));

    // Clone the sender for the webhook handler
    let webhook_event_sender = event_sender.clone();

    // --- Spawn the background price updater ---
    // Create an array of tokens. Initially, we'll include only Ethereum.
    let api_key =
        env::var("ALCHEMY_API_KEY").expect("ALCHEMY_API_KEY environment variable not set");
    let pool_clone = pool.clone();

    // Start HTTP server
    HttpServer::new(move || {
        let cors = Cors::default()
            //only backend cron server and goldsky webhook
            .allowed_origin("https://mopoprotocol.xyz")
            .allowed_origin("https://www.mopoprotocol.xyz")
            .allowed_origin("https://cult-nextjs-l1go.vercel.app/")
            .allowed_origin_fn(|origin, _req_head| {
                origin == "https://cultdottrade-reputationplatform-nex.vercel.app"
            }).allowed_origin_fn(|origin, _req_head| {
                origin == "https://cultdottrade-reputationplatform-nex-ruby.vercel.app"
            })
            .allowed_origin_fn(|origin, _req_head| {
                origin.as_bytes().starts_with(b"http://mopoprotocol.xyz")
            })
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                header::AUTHORIZATION,
                header::ACCEPT,
                header::CONTENT_TYPE,
            ])
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(Logger::new("%a \"%r\" %s %b %T").log_target("error"))
            .wrap(Logger::default())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(webhook_config.clone()))
            .app_data(web::Data::new(sender.clone()))
            .app_data(web::Data::new(webhook_event_sender.clone()))
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-doc/openapi.json", routes::ApiDoc::openapi()),
            )
            .service(
                web::scope("/api")
                    .wrap(NormalizePath::trim())
                    .service(routes::create_account)
                    .service(routes::get_account)
                    .service(routes::get_account_rank)
                    .service(
                        web::resource("/diamond_hands")
                            .route(web::get().to(routes::get_diamond_hands)),
                    )
                    .service(routes::get_merkle_proof)
                    .service(routes::get_all_communities)
                    .service(routes::get_account_communities)
                    .service(routes::get_top_accounts_ranking)
            )
            .service(
                web::scope("/admin")
                    .wrap(NormalizePath::trim())
                    .wrap(ApiGuard::new())
                    .service(
                        web::resource("/run-diamond-hands")
                            .route(web::post().to(diamond_hands_handler)),
                    )
                    .service(
                        web::resource("/run-custom-diamond-hands")
                            .route(web::post().to(custom_diamond_hands_handler)),
                    )
                    .service(
                        web::resource("/run-custom-diamond-hands-csv")
                            .route(web::post().to(custom_diamond_hands_csv_handler)),
                    )
                    .service(
                        web::resource("/run-community-airdrops")
                            .route(web::post().to(community_airdrops_handler)),
                    )
                    .service(
                        web::resource("/run-community-scores")
                            .route(web::post().to(update_all_community_scores)),
                    )
                    .service(
                        web::resource("/run-token-stats")
                            .route(web::post().to(run_token_stats_handler)),
                    )
                    .service(
                        web::resource("/add-upcoming-token")
                            .route(web::post().to(add_upcoming_token_handler)),
                        )
                    .service(
                        web::resource("/update-token-balance")
                            .route(web::post().to(update_token_balance_handler)),
                    )
                    .service(
                        web::resource("/update-token-balance-csv")
                            .route(web::post().to(update_token_balance_from_csv_handler)),
                    )
                    .service(
                        web::resource("/cleanup-duplicate-accounts")
                            .route(web::post().to(cleanup_duplicate_accounts_handler)),
                    ),
                )
            .service(
                web::scope("/webhook")
                    .service(
                        web::resource("/transfer-token").route(
                            web::post()
                                .guard(guard::Header("content-type", "application/json"))
                                .to(transfer_token_webhook_handler),
                        ),
                    )
                    .service(
                        web::resource("/create-token").route(
                            web::post()
                                .guard(guard::Header("content-type", "application/json"))
                                .to(create_token_webhook_handler),
                        ),
                    )
                    .service(
                        web::resource("/swap-token").route(
                            web::post()
                                .guard(guard::Header("content-type", "application/json"))
                                .to(swap_webhook_handler),
                        ),
                    )
            )
    })
    .bind(server_address)?
    .workers(8)
    .run()
    .await
}
