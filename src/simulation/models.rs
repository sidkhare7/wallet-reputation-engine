use ethers::types::{Address, H256, U256};
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};

/// Market type enum to match the smart contract
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MarketType {
    BondingCurve = 0,
    UniswapPool = 1,
}

impl ToString for MarketType {
    fn to_string(&self) -> String {
        match self {
            MarketType::BondingCurve => "bonding_curve".to_string(),
            MarketType::UniswapPool => "uniswap_pool".to_string(),
        }
    }
}

/// Configuration for a simulation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub contract_address: String,
    pub rpc_url: String,
    pub num_accounts: i32,
    pub concurrent_transactions: i32,
    pub min_eth_amount: f64,  // in ETH
    pub max_eth_amount: f64,  // in ETH
    pub market_type: MarketType,
    pub retry_count: i32,
    pub referral_chance: i32,  // 0-100 percentage
    pub comment: Option<String>,
}

/// Response when starting a simulation
#[derive(Debug, Serialize, Deserialize)]
pub struct SimulationResponse {
    pub id: i32,
    pub status: SimulationStatus,
    pub message: String,
}

/// Status of a simulation
#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum SimulationStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl ToString for SimulationStatus {
    fn to_string(&self) -> String {
        match self {
            SimulationStatus::Pending => "pending".to_string(),
            SimulationStatus::Running => "running".to_string(),
            SimulationStatus::Completed => "completed".to_string(),
            SimulationStatus::Failed => "failed".to_string(),
        }
    }
}

/// Database model for a simulation
#[derive(Debug, Serialize, Deserialize)]
pub struct Simulation {
    pub id: i32,
    pub config: serde_json::Value,
    pub total_transactions: i32,
    pub successful_transactions: i32,
    pub failed_transactions: i32,
    pub total_eth_spent: String,
    pub total_fees_paid: String,
    pub total_tokens_received: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Database model for a transaction within a simulation
#[derive(Debug, Serialize, Deserialize)]
pub struct SimulationTransaction {
    pub id: i32,
    pub simulation_id: i32,
    pub tx_hash: Option<String>,
    pub from_address: String,
    pub to_address: String,
    pub eth_amount: String,
    pub tokens_received: Option<String>,
    pub fee_paid: String,
    pub market_type: String,
    pub success: bool,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Result of a buy transaction
#[derive(Debug, Clone)]
pub struct TransactionResult {
    pub tx_hash: Option<H256>,
    pub from_address: Address,
    pub to_address: Address,
    pub eth_amount: U256,
    pub tokens_received: Option<U256>,
    pub fee_paid: U256,
    pub market_type: MarketType,
    pub success: bool,
    pub error: Option<String>,
}

/// Statistics for a simulation run
#[derive(Debug, Default)]
pub struct SimulationStats {
    pub total_transactions: i32,
    pub successful_transactions: i32,
    pub failed_transactions: i32,
    pub total_eth_spent: U256,
    pub total_fees_paid: U256,
    pub total_tokens_received: U256,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            contract_address: "".to_string(),
            rpc_url: "http://localhost:8545".to_string(),
            num_accounts: 100,
            concurrent_transactions: 10,
            min_eth_amount: 0.0000001,
            max_eth_amount: 1.0,
            market_type: MarketType::BondingCurve,
            retry_count: 3,
            referral_chance: 20,  // 20% chance
            comment: Some("Simulation buy".to_string()),
        }
    }
}

