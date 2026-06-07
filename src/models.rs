//use alloy::primitives::u128;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

//////////////
// WEBHOOKS //
//////////////
#[derive(Clone)]
pub struct WebhookConfig {
    pub secret_key: String,
    pub max_concurrent_jobs: usize,
    pub job_queue_buffer: usize,
}

// Webhook payload structures
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WebhookEventType {
    CultTokenCreated,
    // CultTokenBuy,
    // CultTokenSell,
    // TokenClaimed,
    // CultMarketGraduated,
    CultTokenTransfer,
    CultSwap,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebhookPayload {
    pub id: String,
    pub event_type: WebhookEventType,
    pub data: serde_json::Value,
    pub timestamp: i64,
}

// Response for webhook receipt
#[derive(Serialize)]
pub struct WebhookResponse {
    pub status: String,
    pub event_id: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CultToken {
    pub id: Option<String>, // Changed to String for address
    pub factory_address: String,
    pub token_creator: String,
    pub protocol_fee_recipient: String,
    pub bonding_curve: String,
    pub token_uri: String,
    pub name: String,
    pub symbol: String,
    pub pool_address: String,
    pub block_number: i64,
    pub block_timestamp: DateTime<Utc>,
    pub transaction_hash: String,
    pub holder_count: u32,
    pub airdrop_contract: String,
    pub trades: Option<Vec<TokenTrade>>,
    pub balances: Option<Vec<TokenBalance>>,
    pub ipfs_content: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "trade_type", rename_all = "PascalCase")]
pub enum TradeType {
    Buy,
    Sell,
    Claim,
}

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Account {
    pub id: Option<String>, // Changed to String for address
    pub slug: Option<String>,
    pub referral_code: Option<String>,
    pub diamond_hand_probability: u32,
    pub reputation: Option<i32>, // Reputation score ranging from 0-100,000
    pub referrer_id: Option<String>, // Reference to another Account (foreign key)
    pub total_referrals: Option<u32>,
    pub fee_collected: u128,
    pub twitter: Option<String>,
    pub discord: Option<String>,
    pub tokens_created: u32,
    pub tokens_migrated: u32,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TokenBalance {
    pub id: Option<String>, // Changed to String for concatenated hash and address
    pub token_id: String,   // Reference to CultToken (foreign key)
    pub account_id: String, // Reference to Account (foreign key)
    pub value: i64,
    pub last_bought: DateTime<Utc>,
    pub last_sold: DateTime<Utc>,
    pub held_for: i64,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TokenTrade {
    pub id: Option<String>, // Changed to String for concatenated hash and address
    pub token_id: String,   // Reference to CultToken (foreign key)
    pub trade_type: TradeType,
    pub trader_id: String,         // Reference to Account (foreign key)
    pub recipient_id: String,      // Reference to Account (foreign key)
    pub order_referrer_id: String, // Reference to Account (foreign key)
    pub total_eth: i64,
    pub eth_fee: i64,
    pub eth_amount: i64,
    pub token_amount: i64,
    pub trader_token_balance: i64,
    pub total_supply: i64,
    pub market_type: i64,
    pub timestamp: DateTime<Utc>,
    pub transaction_hash: String,
}

#[derive(Debug, Deserialize, Serialize, utoipa::IntoParams)]
pub struct PaginationParams {
    pub offset: i64,
    pub limit: i64,
    /// Sort order (asc or desc)
    #[serde(default = "default_sort_order")]
    pub order: Option<String>,
}

fn default_sort_order() -> Option<String> {
    Some("desc".to_string())
}

pub enum SortColumn {
    BlockTimestamp,
    MarketCap,
    MarketCap24hr,
    Volume24h,
}

impl SortColumn {
    pub fn as_str(&self) -> &'static str {
        match self {
            SortColumn::BlockTimestamp => "block_timestamp",
            SortColumn::MarketCap => "market_cap",
            SortColumn::MarketCap24hr => "market_cap_24hr",
            SortColumn::Volume24h => "volume_24h",
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct CreateAccountRequest {
    pub user_id: String,
    pub referral_code: Option<String>,
    pub twitter: Option<String>,
    pub discord: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CreateAccountResponse {
    pub user_id: String,
    pub referral_code: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WatchlistActionRequest {
    pub account_id: String,
    pub cult_token_id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAccountRequest {
    pub twitter: Option<String>,
    pub discord: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommunityResponse {
    pub name: String,
    pub img_url: String,
    pub chain: String,
    pub merkle_root: String,
    pub holder_count: i64,
    pub community_score: Option<f32>,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema, FromRow)]
pub struct CultTokensResponse {
    pub id: String,
    pub token_creator: String,
    pub name: String,
    pub symbol: String,
    pub ipfs_content: String,
    pub holder_count: u32,
    pub market_cap: f64,
    pub market_cap_24hr: f64,
    pub volume: f64,
    pub volume_24h: f64,
    pub total_airdrop_recipient_count: u32,
    pub creator_holdings: f64,
    pub top_holders: f64,
    pub buy_tx_count_24h: u32,
    pub sell_tx_count_24h: u32,
    #[schema(value_type = Option<String>, example = "2023-01-01T00:00:00Z")]
    pub last_traded: Option<DateTime<Utc>>, // Changed to Option
    pub bonding_curve_percentage: f64,
    pub is_graduated: bool,
    #[schema(value_type = String, example = "2023-01-01T00:00:00Z")]
    pub block_timestamp: DateTime<Utc>,
    pub pool_address: String,
    pub is_watchlisted: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpcomingTokensResponse {
    pub id: String,
    pub token_creator: String,
    pub name: String,
    pub symbol: String,
    pub ipfs_content: String,
    #[schema(value_type = Option<String>, example = "2023-01-01T00:00:00Z")]
    pub release_date: Option<DateTime<Utc>>,
    pub total_airdrop_recipient_count: Option<i64>, // Or Option<u32> if that's more appropriate
    pub community_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, FromRow)]
pub struct UpcomingTokenPayload {
    pub id: String,
    pub token_creator: String,
    pub token_uri: String,
    pub name: String,
    pub symbol: String,
    #[schema(value_type = String, example = "2023-01-01T00:00:00Z")]
    pub release_date: DateTime<Utc>,
    pub total_airdrop_recipient_count: Option<i64>,
    pub ipfs_content: String,
    pub community_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CultTokenTopHolder {
    pub value: f64,
    pub id: String,
    pub slug: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, utoipa::IntoParams)]
pub struct TopHolderParams {
    pub token_address: String,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CultTokenDataResponse {
    pub id: String,
    pub token_creator: String,
    pub name: String,
    pub symbol: String,
    pub ipfs_content: String,
    pub holder_count: u32,
    pub market_cap: f64,
    pub market_cap_24hr: f64,
    pub volume: f64,
    pub volume_24h: f64,
    pub total_airdrop_recipient_count: u32,
    pub creator_holdings: f64,
    pub top_holders: f64,
    pub buy_tx_count_24h: u32,
    pub sell_tx_count_24h: u32,
    #[schema(value_type = Option<String>, example = "2023-01-01T00:00:00Z")]
    pub last_traded: Option<DateTime<Utc>>,
    pub bonding_curve_percentage: f64,
    pub is_graduated: bool,
    #[schema(value_type = String, example = "2023-01-01T00:00:00Z")]
    pub block_timestamp: DateTime<Utc>,
    pub pool_address: String,
    pub airdrop_contract: String,
    pub trade_count: u32,
    pub is_watchlisted: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, FromRow, ToSchema)]
pub struct TokenTradesResponse {
    pub id: String,
    pub trader: String,
    pub recipient: String,
    pub order_referer: String,
    pub eth_amount: String,
    pub token_amount: String,
    pub trader_token_balance: String,
    pub market_type: i64,
    pub trade_type: TradeType,
    #[schema(value_type = String, example = "2023-01-01T00:00:00Z")]
    pub timestamp: DateTime<Utc>,
    pub transaction_hash: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccountDetailResponse {
    pub id: String,
    pub slug: Option<String>,
    pub diamond_hand_probability: Option<i32>, // ✅ changed
    pub total_referrals: Option<i32>,          // ✅ changed
    pub fee_collected: Option<BigDecimal>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AirdroppedToken {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub ipfs_content: String,
    pub is_graduated: bool,
    pub pool_address: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountData {
    pub created_tokens: Vec<CreatedToken>,
    pub owned_tokens: Vec<OwnedToken>,
    pub watchlist: Vec<WatchlistToken>,
    pub communities: Vec<Community>,
    pub airdropped_tokens: Vec<AirdroppedToken>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatedToken {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub ipfs_content: String,
    pub user_balance: String, // Wei as string
    pub is_graduated: bool,
    pub pool_address: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OwnedToken {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub ipfs_content: String,
    pub user_balance: String, // Wei as string
    pub is_graduated: bool,
    pub pool_address: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WatchlistToken {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub ipfs_content: String,
    pub user_balance: String, // Wei as string
    pub is_graduated: bool,
    pub pool_address: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Community {
    pub id: String,
    pub name: String,
    pub img_url: String,
}

// Request and response structs for merkle proof
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct MerkleProofResponse {
    pub token_id: String,
    pub account_id: String,
    pub merkle_root: String,             // Hex-encoded
    pub merkle_proof: serde_json::Value, // Keeping as JSON Value for flexibility
    pub community_id: String,
    pub community_name: String,
    pub total_amount: String, // Using String for large numeric values
    pub transaction_hash: String,
    pub claimed: bool,
}

#[derive(Debug, FromRow, ToSchema, Serialize)]
pub struct CryptoPrice {
    #[schema(example = "ETH")]
    pub symbol: String,
    #[schema(example = "1583.6979679075")]
    pub price: f64,
    #[schema(example = "2.5")]
    pub change_24h: f64,
    #[schema(example = "0.5")]
    pub change_1h: f64,
    #[schema(value_type = String, example = "2023-01-01T00:00:00Z")]
    pub last_updated_at: DateTime<Utc>,
}

// #[derive(Debug, Deserialize, Serialize, FromRow, ToSchema)]
// pub struct AccountCommunitiesResponse {
//     pub id: String,
//     pub name: String,
//     pub description: Option<String>,
//     pub address: String,
//     pub member_count: i64,
// }

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct DiamondHandsRequest {
    pub addresses: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct DiamondHandsResponse {
    pub message: String,
    pub accounts_processed: usize,
    pub accounts_created: usize,
    pub diamond_hands_updated: bool,
}
