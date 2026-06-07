// use alloy_primitives::{Address, U256, U64, Bytes};
// use alloy_provider::{Provider, ProviderBuilder};
// use alloy_rpc_client::RpcClient;
// use alloy_transport_http::Http;
// use alloy_signer::{k256::ecdsa::SigningKey, LocalWallet, Signer};
// use alloy_contract::{Contract, ContractInstance};
// use alloy_json_abi::{Function, JsonAbi};
// use alloy_network::{Ethereum, Network};
// use alloy_consensus::SignableTransaction;
// use alloy_genesis::{Genesis, GenesisAccount};

use alloy::{
    contract::{ContractInstance, Interface},
    network::{EthereumWallet, Ethereum},
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
    primitives::{Address, Bytes, B256, U256},
    rpc::types::TransactionRequest,
    dyn_abi::DynSolValue,
    rpc::client::ClientBuilder,
    transports::http::Http,
    json_abi::{JsonAbi, Function},
    sol
};

use std::{
    sync::{Arc, Mutex}, 
    collections::HashMap,
    str::FromStr,
    time::Duration,
    fs::File,
    io::Write,
};
use tokio::{
    sync::{Semaphore, mpsc}, 
    time::sleep,
};
use rand::{Rng, thread_rng, seq::SliceRandom};
use chrono::Local;
use serde::{Serialize, Deserialize};
use csv::Writer;
use anyhow::{Result, anyhow};
use futures::future::join_all;
use std::env;
use dotenv::dotenv;
mod db;
mod abi;
use abi::{sell_calldata, buy_calldata};
use db::{EnhancedAccount, TokenBalance, DbClient};

// Constants
const MIN_ORDER_SIZE: f64 = 0.0000001;
const TOTAL_FEE_BPS: u64 = 100; // 1%
const REFERRAL_CHANCE: u8 = 20; // 20% chance of using a referral
const DEFAULT_RPC_URL: &str = "http://localhost:8545";
const DEFAULT_CULT_TOKEN_ADDRESS: &str = "0x0000000000000000000000000000000000000000"; // Replace with actual address
const DEFAULT_NUM_ACCOUNTS: usize = 100;
const DEFAULT_CONCURRENCY: usize = 10;
const DEFAULT_TX_COUNT: usize = 1000;
const DEFAULT_MAX_RETRIES: usize = 3;
const RETRY_DELAY_MS: u64 = 1000;

// Market type enum
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
enum MarketType {
    BondingCurve = 0,
    UniswapPool = 1,
}

// Transaction type enum
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
enum TransactionType {
    Buy,
    Sell,
}

// Transaction result struct
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionResult {
    tx_hash: String,
    account: String,
    tx_type: TransactionType,
    market_type: MarketType,
    eth_amount: f64,
    token_amount: f64,
    fee_amount: f64,
    success: bool,
    error: Option<String>,
    timestamp: String,
    block_number: Option<u64>,
    referral_used: bool,
}

// Aggregate statistics struct
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SimulationStats {
    total_transactions: usize,
    successful_transactions: usize,
    failed_transactions: usize,
    success_rate: f64,
    total_eth_spent: f64,
    total_fees_collected: f64,
    total_tokens_received: f64,
    average_tx_size_eth: f64,
    average_tx_size_tokens: f64,
    bonding_curve_txs: usize,
    uniswap_pool_txs: usize,
    buy_txs: usize,
    sell_txs: usize,
}

// Simulation configuration
#[derive(Debug, Clone)]
struct SimulationConfig {
    rpc_url: String,
    database_url: String,
    cult_token_address: Address,
    cult_token_id: String,
    account_count: usize,
    concurrency: usize,
    transaction_count: usize,
    max_retries: usize,
    buy_only: bool,
    sell_only: bool,
    market_type_override: Option<MarketType>,
    output_csv: String,
    log_file: String,
    accounts_file: String,
}

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Cult,
    "contracts/abis/Cult.json"
);

pub fn build_tx(to: Address, from: Address, calldata: Bytes, base_fee: u128) -> TransactionRequest {
    TransactionRequest::default()
        .to(to)
        .from(from)
        .input(calldata)
        .nonce(0)
        .gas_limit(1000000)
        .max_fee_per_gas(base_fee)
        .max_priority_fee_per_gas(0)
        .build_unsigned()
        .unwrap()
        .into()
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            rpc_url: DEFAULT_RPC_URL.to_string(),
            database_url: "postgres://postgres:postgres@localhost:5432/cult_db".to_string(),
            cult_token_address: Address::from_str(DEFAULT_CULT_TOKEN_ADDRESS)
                .expect("Invalid default token address"),
            cult_token_id: "default_token_id".to_string(),
            account_count: DEFAULT_NUM_ACCOUNTS,
            concurrency: DEFAULT_CONCURRENCY,
            transaction_count: DEFAULT_TX_COUNT,
            max_retries: DEFAULT_MAX_RETRIES,
            buy_only: false,
            sell_only: false,
            market_type_override: None,
            output_csv: "cult_token_simulation.csv".to_string(),
            log_file: "cult_token_simulation.log".to_string(),
            accounts_file: "accounts.json".to_string(),
        }
    }
}

// The main simulator struct
struct CultTokenSimulator {
    config: SimulationConfig,
    provider: Arc<Box<dyn Provider<Ethereum>>>,
    cult_token_abi: JsonAbi,
    cult_contract: ContractInstance<Arc<Box<dyn Provider<Ethereum>>>, Ethereum>,
    enhanced_accounts: Vec<EnhancedAccount>,
    token_balances: HashMap<String, TokenBalance>,
    results: Arc<Mutex<Vec<TransactionResult>>>,
    logger: Arc<Mutex<File>>,
    csv_writer: Arc<Mutex<Writer<File>>>,
    semaphore: Arc<Semaphore>,
}

impl CultTokenSimulator {
    // Create a new simulator instance
    async fn new(config: SimulationConfig) -> Result<Self> {
        // Set up Ethereum provider
        let provider = ProviderBuilder::new().on_http(std::env::var("MONAD_TESTNET_RPC_URL").unwrap().parse()?);
        let provider = Box::new(provider) as Box<dyn Provider<Ethereum>>;
        let provider = Arc::new(provider);
        
        // Load the ABI
        let cult_token_abi = Self::load_cult_token_abi()?;
        let cult_contract = ContractInstance::new(config.cult_token_address, provider.clone(), Interface::new(cult_token_abi.clone()));
        
        // Load accounts and token balances from database
        let (enhanced_accounts, token_balances) = Self::load_accounts(
            config.account_count, 
            &config.database_url
        ).await?;

        // Setup CSV output
        let csv_file = File::create(&config.output_csv)?;
        let csv_writer = Writer::from_writer(csv_file);
        
        // Setup log file
        let log_file = File::create(&config.log_file)?;
        

        // Create the simulator
        let simulator: CultTokenSimulator = Self {
            config:config.clone(),
            provider,
            cult_token_abi,
            cult_contract,
            enhanced_accounts,
            token_balances,
            results: Arc::new(Mutex::new(Vec::new())),
            logger: Arc::new(Mutex::new(log_file)),
            csv_writer: Arc::new(Mutex::new(csv_writer)),
            semaphore: Arc::new(Semaphore::new(config.clone().concurrency)),
        };
        
        Ok(simulator)
    }
    
    // Load the ABI for the Cult Token contract
    fn load_cult_token_abi() -> Result<JsonAbi> {
        // This should load the ABI from a file or hardcode it
        // For now, we'll use a placeholder
    // Get the contract ABI.
        let path = std::env::current_dir()?.join("contracts/abis/Cult.json");

        // Read the artifact which contains `abi`, `bytecode`, `deployedBytecode` and `metadata`.   
        let artifact = std::fs::read(path).expect("Failed to read artifact");
        let json: serde_json::Value = serde_json::from_slice(&artifact)?;

    // Get `abi` from the artifact.
        let abi_value = json.get("abi").expect("Failed to get ABI from artifact");
        let abi = serde_json::from_str(&abi_value.to_string())?;
        Ok(abi)
    }
    
    async fn load_accounts(
        account_count: usize,
        database_url: &str
    ) -> Result<(Vec<EnhancedAccount>, HashMap<String, TokenBalance>)> {
        println!("Loading accounts from database...");
        
        // Create a new database client
        let db_client = DbClient::new(database_url).await?;
        
        // Load enhanced accounts from the database using accounts.json file
        let accounts_file = std::env::current_dir()?.join("contracts/accounts.json");
        let enhanced_accounts = db_client.load_accounts_from_file_and_db(
            accounts_file.to_str().unwrap_or("accounts.json"), 
            account_count
        ).await?;
        
        println!("Successfully loaded {} accounts from database", enhanced_accounts.len());
        
        // Get account IDs for token balance lookup
        let account_ids: Vec<String> = enhanced_accounts.iter()
            .map(|account| account.account_id.clone())
            .collect();
        
        // Load token balances for these accounts
        // Assuming the token ID is specified in the environment or config
        // Here we're using a placeholder token ID - you should replace this with your actual token ID
        let token_id = std::env::var("CULT_TOKEN_ID").unwrap_or_else(|_| "cult_token".to_string());
        let token_balances = db_client.get_token_balances(&token_id, &account_ids).await?;
        
        println!("Successfully loaded {} token balances", token_balances.len());
        
        Ok((enhanced_accounts, token_balances))
    }


// // Generate Ethereum accounts from a JSON file
// fn generate_accounts(count: usize) -> Result<Vec<LocalWallet>> {
//     let mut accounts = Vec::with_capacity(count);
    
//     // Try to read accounts from the JSON file
//     let json_path = std::env::current_dir()?.join("contracts/accounts.json");
    
//     if json_path.exists() {
//         // Read the file
//         let accounts_data = std::fs::read_to_string(json_path)?;
        
//         // Parse the JSON
//         let accounts_json: Vec<String> = serde_json::from_str(&accounts_data)?;
        
//         // Take only the requested number of accounts
//         for (i, private_key) in accounts_json.iter().enumerate() {
//             if i >= count {
//                 break;
//             }
            
//             // Parse the private key and create a wallet
//             let signing_key = SigningKey::from_bytes(&hex::decode(private_key.trim_start_matches("0x"))?)?;
//             let wallet = LocalWallet::from(signing_key);
//             accounts.push(wallet);
//         }
        
//         // Log how many accounts were loaded
//         println!("Loaded {} accounts from accounts.json", accounts.len());
        
//         // If we didn't get enough accounts, generate random ones to make up the difference
//         if accounts.len() < count {
//             println!("Generating {} additional random accounts", count - accounts.len());
//             let mut rng = thread_rng();
//             for _ in accounts.len()..count {
//                 let wallet = LocalWallet::random(&mut rng);
//                 accounts.push(wallet);
//             }
//         }
//     } else {
//         // If the file doesn't exist, generate random accounts
//         println!("accounts.json not found, generating {} random accounts", count);
//         let mut rng = thread_rng();
//         for _ in 0..count {
//             let wallet = LocalWallet::random(&mut rng);
//             accounts.push(wallet);
//         }
//     }
    
//     Ok(accounts)
// }
    
    
    // Log a message to the log file
    fn log(&self, message: &str) -> Result<()> {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let log_message = format!("[{}] {}\n", timestamp, message);
        
        println!("{}", log_message.trim());
        
        let mut logger = self.logger.lock().unwrap();
        logger.write_all(log_message.as_bytes())?;
        logger.flush()?;
        
        Ok(())
    }
    
    // Get the current market type from the contract
    async fn get_current_market_type(&self) -> Result<MarketType> {
        //TODO: Can fix this currently defaulting to bonding curve
        // let contract = ContractInstance::new(self.config.cult_token_address, self.provider.clone(), Interface::new(self.cult_token_abi.clone()));
        
        // // Find the marketType function in the ABI
        // let market_type_fn = self.cult_token_abi.functions().find(|f| f.name == "marketType")
        //     .ok_or_else(|| anyhow!("marketType function not found in ABI"))?;
        
        // // Call the marketType function
        // let result = client.call(
        //     &contract.encode_call(market_type_fn, &[]),
        //     self.config.cult_token_address,
        //     None,
        //     None,
        //     None,
        // ).await?;
        
        // // Decode the result
        // let market_type: u8 = result.value().to_scalar().unwrap().as_u8();
        
        // match market_type {
        //     0 => Ok(MarketType::BondingCurve),
        //     1 => Ok(MarketType::UniswapPool),
        //     _ => Err(anyhow!("Unknown market type: {}", market_type)),
        // }

        Ok(MarketType::BondingCurve)
    }
    
    // Execute a buy transaction
    async fn execute_buy_transaction(
        &self,
        enhanced_account: &EnhancedAccount,
        market_type: MarketType,
    ) -> Result<TransactionResult> {
        let provider = self.provider.clone();
        let mut rng = thread_rng();
        
        // Random ETH amount between MIN_ORDER_SIZE and 0.1 ETH
        let eth_amount = rng.gen_range(MIN_ORDER_SIZE..0.1);
        let eth_value = U256::from((eth_amount * 1e18) as u64);
        
        let (use_referral, referral_address) = if let Some(referrer_id) = &enhanced_account.referrer_id {
            // Convert string address to Address type
            match Address::from_str(referrer_id) {
                Ok(address) => (true, address),
                Err(_) => (false, Address::ZERO),
            }
        } else {
            (false, Address::ZERO)
        };
        
        
        // Get nonce for the account
        let wallet = &enhanced_account.wallet;
        let nonce = provider.get_transaction_count(wallet.address()).await?;
        let base_fee = provider.get_gas_price().await?;

        // Find the buy function in the ABI
        let buy_fn = self.cult_token_abi.functions().find(|f| f.name == "buy")
            .ok_or_else(|| anyhow!("buy function not found in ABI"))?;
        

        let calldata = buy_calldata(&wallet.address(), &wallet.address(), &referral_address,  "Simulation buy".to_string(), market_type, U256::ZERO, U256::ZERO);
        

        let tx = build_tx(self.config.cult_token_address, wallet.address(), calldata, base_fee);
        
        // Get the chain ID
        let chain_id = provider.get_chain_id().await?;
        
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        
        // Execute the transaction with retry logic
        for attempt in 0..self.config.max_retries {
            // Get gas price and estimate gas
            let gas_price = provider.get_gas_price().await?;
            
            // Send the transaction
            match provider.send_raw_transaction(signed_tx.as_ref().to_vec().into()).await {
                Ok(tx_hash) => {
                    // Wait for the transaction to be mined
                    match provider.get_transaction_receipt(tx_hash).await {
                        Ok(Some(receipt)) => {
                            // Calculate token amount and fee based on the receipt
                            // This is a simplification - would need proper event parsing
                            let token_amount = eth_amount * 100.0; // Simplified conversion
                            let fee_amount = eth_amount * (TOTAL_FEE_BPS as f64) / 10000.0;
                            
                            return Ok(TransactionResult {
                                tx_hash: format!("{:?}", tx_hash),
                                account: format!("{:?}", wallet.address()),
                                tx_type: TransactionType::Buy,
                                market_type,
                                eth_amount,
                                token_amount,
                                fee_amount,
                                success: receipt.status.unwrap_or(U64::ZERO) == U64::from(1),
                                error: None,
                                timestamp,
                                block_number: receipt.block_number.map(|b| b.to::<u64>()),
                                referral_used: use_referral,
                            });
                        },
                        Ok(None) => {
                            return Ok(TransactionResult {
                                tx_hash: "pending".to_string(),
                                account: format!("{:?}", wallet.address()),
                                tx_type: TransactionType::Buy,
                                market_type,
                                eth_amount,
                                token_amount: 0.0,
                                fee_amount: 0.0,
                                success: false,
                                error: Some("Transaction pending too long".to_string()),
                                timestamp,
                                block_number: None,
                                referral_used: use_referral,
                            });
                        },
                        Err(e) => {
                            if attempt < self.config.max_retries - 1 {
                                self.log(&format!("Attempt {}: Transaction failed, retrying: {}", attempt + 1, e))?;
                                sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                                continue;
                            }
                            
                            return Ok(TransactionResult {
                                tx_hash: "failed".to_string(),
                                account: format!("{:?}", wallet.address()),
                                tx_type: TransactionType::Buy,
                                market_type,
                                eth_amount,
                                token_amount: 0.0,
                                fee_amount: 0.0,
                                success: false,
                                error: Some(format!("Transaction confirmation error: {}", e)),
                                timestamp,
                                block_number: None,
                                referral_used: use_referral,
                            });
                        }
                    }
                },
                Err(e) => {
                    if attempt < self.config.max_retries - 1 {
                        self.log(&format!("Attempt {}: Failed to send transaction, retrying: {}", attempt + 1, e))?;
                        sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                        continue;
                    }
                    
                    return Ok(TransactionResult {
                        tx_hash: "failed".to_string(),
                        account: format!("{:?}", wallet.address()),
                        tx_type: TransactionType::Buy,
                        market_type,
                        eth_amount,
                        token_amount: 0.0,
                        fee_amount: 0.0,
                        success: false,
                        error: Some(format!("Failed to send transaction: {}", e)),
                        timestamp,
                        block_number: None,
                        referral_used: use_referral,
                    });
                }
            }
        }
        
        // Should never reach here due to the retry loop
        unreachable!()
    }
    
    // Execute a sell transaction
    async fn execute_sell_transaction(
        &self,
        enhanced_account: &EnhancedAccount,
        market_type: MarketType,
    ) -> Result<TransactionResult> {
        let provider = self.provider.clone();
        let mut rng = thread_rng();
        
        // This would need to check token balance in reality
        let sell_value = match self.token_balances.get(&enhanced_account.account_id) {
            Some(balance) => {
                // Convert BigDecimal to f64 for calculation
                let holdings_value = balance.holdings_value.to_string();
                let value = holdings_value.parse::<f64>().unwrap_or(0.0);
                let sell_percentage = rng.gen_range(0.01..0.9);
                
                // Calculate tokens to sell based on balance and sell percentage
                value * sell_percentage
            },
            None => {
                0.0
            }
        };
        
        let tokens_to_sell = U256::from((sell_value * 1e18) as u64);
        
        // 20% chance of using a referral
        let (use_referral, referral_address) = if let Some(referrer_id) = &enhanced_account.referrer_id {
            // Convert string address to Address type
            match Address::from_str(referrer_id) {
                Ok(address) => (true, address),
                Err(_) => (false, Address::ZERO),
            }
        } else {
            (false, Address::ZERO)
        };
        
        // Get wallet from enhanced account
        let wallet = &enhanced_account.wallet;
        
        // Get nonce for the account
        let nonce = provider.get_transaction_count(wallet.address(), None).await?;
        
        // Find the sell function in the ABI
        let sell_fn = self.cult_token_abi.functions().find(|f| f.name == "sell")
            .ok_or_else(|| anyhow!("sell function not found in ABI"))?;
        
        // Create the sell transaction data
        let data = alloy_contract::encode_function_data(
            sell_fn,
            &[
                tokens_to_sell.into(), // tokensToSell
                alloy_primitives::Selector::from_slice(&wallet.address().to_vec()).into(), // recipient
                alloy_primitives::Selector::from_slice(&referral_address.to_vec()).into(), // orderReferrer
                "Simulation sell".to_string().into(), // comment
                (market_type as u8).into(), // expectedMarketType
                U256::ZERO.into(), // minPayoutSize
                U256::ZERO.into(), // sqrtPriceLimitX96
            ],
        )?;
        
        // Create the transaction request
        let mut tx = alloy_rpc_types::TransactionRequest::default();
        tx.from = Some(wallet.address());
        tx.to = Some(self.config.cult_token_address);
        tx.data = Some(data.into());
        tx.nonce = Some(nonce);
        
        // Get the chain ID
        let chain_id = provider.get_chain_id().await?;
        
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        
        // Execute the transaction with retry logic
        for attempt in 0..self.config.max_retries {
            // Get gas price and estimate gas
            let gas_price = provider.get_gas_price().await?;
            tx.gas_price = Some(gas_price);
            
            let gas_estimate = provider.estimate_gas(&tx, None).await?;
            tx.gas = Some(gas_estimate);
            
            // Sign the transaction
            let signed_tx = wallet.sign_transaction(&tx, chain_id).await?;
            
            // Send the transaction
            match provider.send_raw_transaction(signed_tx.as_ref().to_vec().into()).await {
                Ok(tx_hash) => {
                    // Wait for the transaction to be mined
                    match provider.get_transaction_receipt(tx_hash).await {
                        Ok(Some(receipt)) => {
                            // Calculate ETH amount and fee based on the receipt
                            // This is a simplification - would need proper event parsing
                            let eth_amount = token_amount / 100.0; // Simplified conversion
                            let fee_amount = eth_amount * (TOTAL_FEE_BPS as f64) / 10000.0;
                            
                            return Ok(TransactionResult {
                                tx_hash: format!("{:?}", tx_hash),
                                account: format!("{:?}", wallet.address()),
                                tx_type: TransactionType::Sell,
                                market_type,
                                eth_amount,
                                token_amount,
                                fee_amount,
                                success: receipt.status.unwrap_or(U64::ZERO) == U64::from(1),
                                error: None,
                                timestamp,
                                block_number: receipt.block_number.map(|b| b.to::<u64>()),
                                referral_used: use_referral,
                            });
                        },
                        Ok(None) => {
                            return Ok(TransactionResult {
                                tx_hash: "pending".to_string(),
                                account: format!("{:?}", wallet.address()),
                                tx_type: TransactionType::Sell,
                                market_type,
                                eth_amount: 0.0,
                                token_amount,
                                fee_amount: 0.0,
                                success: false,
                                error: Some("Transaction pending too long".to_string()),
                                timestamp,
                                block_number: None,
                                referral_used: use_referral,
                            });
                        },
                        Err(e) => {
                            if attempt < self.config.max_retries - 1 {
                                self.log(&format!("Attempt {}: Transaction failed, retrying: {}", attempt + 1, e))?;
                                sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                                continue;
                            }
                            
                            return Ok(TransactionResult {
                                tx_hash: "failed".to_string(),
                                account: format!("{:?}", wallet.address()),
                                tx_type: TransactionType::Sell,
                                market_type,
                                eth_amount: 0.0,
                                token_amount,
                                fee_amount: 0.0,
                                success: false,
                                error: Some(format!("Transaction confirmation error: {}", e)),
                                timestamp,
                                block_number: None,
                                referral_used: use_referral,
                            });
                        }
                    }
                },
                Err(e) => {
                    if attempt < self.config.max_retries - 1 {
                        self.log(&format!("Attempt {}: Failed to send transaction, retrying: {}", attempt + 1, e))?;
                        sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                        continue;
                    }
                    
                    return Ok(TransactionResult {
                        tx_hash: "failed".to_string(),
                        account: format!("{:?}", wallet.address()),
                        tx_type: TransactionType::Sell,
                        market_type,
                        eth_amount: 0.0,
                        token_amount,
                        fee_amount: 0.0,
                        success: false,
                        error: Some(format!("Failed to send transaction: {}", e)),
                        timestamp,
                        block_number: None,
                        referral_used: use_referral,
                    });
                }
            }
        }
        // Should never reach here due to the retry loop
        unreachable!()
    }
    
    // Process a single transaction
    async fn process_transaction(
        &self,
        wallet: LocalWallet,
        tx_index: usize,
    ) -> Result<()> {
        // Acquire a semaphore permit for rate limiting
        let _permit = self.semaphore.acquire().await?;
        
        // Determine the market type to use
        let market_type = match self.config.market_type_override {
            Some(mt) => mt,
            None => {
                // Get the current market type from the contract or choose randomly
                match self.get_current_market_type().await {
                    Ok(mt) => mt,
                    Err(_) => {
                        // If we can't get the market type, use bonding curve by default
                        MarketType::BondingCurve
                    }
                }
            }
        };
        
        // Determine transaction type (buy or sell)
        let tx_type = if self.config.buy_only {
            TransactionType::Buy
        } else if self.config.sell_only {
            TransactionType::Sell
        } else {
            // Random choice between buy and sell
            if thread_rng().gen_bool(0.5) {
                TransactionType::Buy
            } else {
                TransactionType::Sell
            }
        };
        
        // Log the start of the transaction
        self.log(&format!(
            "Starting transaction {}/{}: {} on {:?} market with account {}",
            tx_index + 1,
            self.config.transaction_count,
            if tx_type == TransactionType::Buy { "BUY" } else { "SELL" },
            market_type,
            wallet.address()
        ))?;
        
        // Execute the transaction based on type
        let result = match tx_type {
            TransactionType::Buy => self.execute_buy_transaction(wallet.clone(), market_type).await,
            TransactionType::Sell => self.execute_sell_transaction(wallet.clone(), market_type).await,
        }?;
        
        // Log the result
        self.log(&format!(
            "Transaction {}/{} complete: {} {} tokens for {} ETH (Fee: {} ETH) - Success: {}",
            tx_index + 1,
            self.config.transaction_count,
            if tx_type == TransactionType::Buy { "Bought" } else { "Sold" },
            result.token_amount,
            result.eth_amount,
            result.fee_amount,
            result.success
        ))?;
        
        // Write to CSV
        {
            let mut writer = self.csv_writer.lock().unwrap();
            writer.serialize(&result)?;
            writer.flush()?;
        }
        
        // Store the result
        {
            let mut results = self.results.lock().unwrap();
            results.push(result);
        }
        
        Ok(())
    }
    
    // Calculate simulation statistics
    fn calculate_stats(&self) -> Result<SimulationStats> {
        let results = self.results.lock().unwrap();
        
        let total_transactions = results.len();
        let successful_transactions = results.iter().filter(|r| r.success).count();
        let failed_transactions = total_transactions - successful_transactions;
        
        let success_rate = if total_transactions > 0 {
            (successful_transactions as f64) / (total_transactions as f64)
        } else {
            0.0
        };
        
        let total_eth_spent: f64 = results.iter()
            .filter(|r| r.success && r.tx_type == TransactionType::Buy)
            .map(|r| r.eth_amount)
            .sum();
        
        let total_fees_collected: f64 = results.iter()
            .filter(|r| r.success)
            .map(|r| r.fee_amount)
            .sum();
        
        let total_tokens_received: f64 = results.iter()
            .filter(|r| r.success && r.tx_type == TransactionType::Buy)
            .map(|r| r.token_amount)
            .sum();
        
        let average_tx_size_eth = if successful_transactions > 0 {
            results.iter()
                .filter(|r| r.success)
                .map(|r| r.eth_amount)
                .sum::<f64>() / (successful_transactions as f64)
        } else {
            0.0
        };
        
        let average_tx_size_tokens = if successful_transactions > 0 {
            results.iter()
                .filter(|r| r.success)
                .map(|r| r.token_amount)
                .sum::<f64>() / (successful_transactions as f64)
        } else {
            0.0
        };
        
        let bonding_curve_txs = results.iter()
            .filter(|r| r.market_type == MarketType::BondingCurve)
            .count();
        
        let uniswap_pool_txs = results.iter()
            .filter(|r| r.market_type == MarketType::UniswapPool)
            .count();
        
        let buy_txs = results.iter()
            .filter(|r| r.tx_type == TransactionType::Buy)
            .count();
        
        let sell_txs = results.iter()
            .filter(|r| r.tx_type == TransactionType::Sell)
            .count();
        
        Ok(SimulationStats {
            total_transactions,
            successful_transactions,
            failed_transactions,
            success_rate,
            total_eth_spent,
            total_fees_collected,
            total_tokens_received,
            average_tx_size_eth,
            average_tx_size_tokens,
            bonding_curve_txs,
            uniswap_pool_txs,
            buy_txs,
            sell_txs,
        })
    }
    
    // Print statistics
    fn print_stats(&self, stats: &SimulationStats) -> Result<()> {
        let stats_str = format!(
            "\n=== SIMULATION STATISTICS ===\n\
             Total Transactions: {}\n\
             Successful Transactions: {} ({:.2}%)\n\
             Failed Transactions: {}\n\
             Total ETH Spent: {:.6} ETH\n\
             Total Fees Collected: {:.6} ETH\n\
             Total Tokens Received: {:.6} CULT\n\
             Average Transaction Size: {:.6} ETH / {:.6} CULT\n\
             Market Type Breakdown:\n\
             - Bonding Curve: {} transactions\n\
             - Uniswap Pool: {} transactions\n\
             Transaction Type Breakdown:\n\
             - Buy: {} transactions\n\
             - Sell: {} transactions\n",
            stats.total_transactions,
            stats.successful_transactions,
            stats.success_rate * 100.0,
            stats.failed_transactions,
            stats.total_eth_spent,
            stats.total_fees_collected,
            stats.total_tokens_received,
            stats.average_tx_size_eth,
            stats.average_tx_size_tokens,
            stats.bonding_curve_txs,
            stats.uniswap_pool_txs,
            stats.buy_txs,
            stats.sell_txs,
        );
        
        println!("{}", stats_str);
        
        // Also log the stats to the log file
        let mut logger = self.logger.lock().unwrap();
        logger.write_all(stats_str.as_bytes())?;
        logger.flush()?;
        
        Ok(())
    }
    
    // Run the simulation
    // Run the simulation
    async fn run(&self) -> Result<SimulationStats> {
        self.log(&format!(
            "Starting Cult Token simulation with {} accounts and {} transactions",
            self.enhanced_accounts.len(),
            self.config.transaction_count
        ))?;
        
        // Log token balance information
        self.log(&format!(
            "Loaded {} token balances from database",
            self.token_balances.len()
        ))?;
        
        // Create tasks for all transactions
        let mut tasks = Vec::with_capacity(self.config.transaction_count);
        
        for i in 0..self.config.transaction_count {
            // Choose an account for each transaction
            // We'll distribute transactions based on account activity metrics
            // Accounts with higher total_referrals get more transactions
            let account_index = if self.enhanced_accounts.len() > 1 {
                // Calculate weighted distribution based on total_referrals
                let total_referrals: i32 = self.enhanced_accounts.iter()
                    .map(|a| a.total_referrals.max(1)) // Ensure minimum weight of 1
                    .sum();
                
                let mut rng = thread_rng();
                let target = rng.gen_range(0..total_referrals);
                
                let mut cumulative = 0;
                let mut selected_index = 0;
                
                for (idx, account) in self.enhanced_accounts.iter().enumerate() {
                    cumulative += account.total_referrals.max(1);
                    if cumulative > target {
                        selected_index = idx;
                        break;
                    }
                }
                
                selected_index
            } else {
                // If there's only one account or none, use modulo
                i % self.enhanced_accounts.len()
            };
            
            let enhanced_account = self.enhanced_accounts[account_index].clone();
            
            let simulator = self.clone();
            let task = tokio::spawn(async move {
                match simulator.process_transaction(&enhanced_account, i).await {
                    Ok(_) => {},
                    Err(e) => {
                        simulator.log(&format!("Error processing transaction {}: {}", i + 1, e)).unwrap_or(());
                    }
                }
            });
            
            tasks.push(task);
        }
        
        // Wait for all tasks to complete
        join_all(tasks).await;
        
        // Calculate and return statistics
        let stats = self.calculate_stats()?;
        self.print_stats(&stats)?;
        
        Ok(stats)
    }
}

impl Clone for CultTokenSimulator {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            provider: self.provider.clone(),
            cult_token_abi: self.cult_token_abi.clone(),
            enhanced_accounts: self.enhanced_accounts.clone(),
            token_balances: self.token_balances.clone(),
            results: self.results.clone(),
            logger: self.logger.clone(),
            csv_writer: self.csv_writer.clone(),
            semaphore: self.semaphore.clone(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();
    
    // Create default configuration
    let mut config = SimulationConfig::default();
    
    // Override with environment variables if present
    if let Ok(rpc_url) = env::var("MONAD_TESTNET_RPC_URL") {
        config.rpc_url = rpc_url;
    }
    
    if let Ok(database_url) = env::var("DATABASE_URL") {
        config.database_url = database_url;
    }
    
    if let Ok(token_address) = env::var("CULT_TOKEN_ADDRESS") {
        config.cult_token_address = Address::from_str(&token_address)?;
    }
    
    if let Ok(token_id) = env::var("CULT_TOKEN_ID") {
        config.cult_token_id = token_id;
    }
    
    if let Ok(account_count) = env::var("ACCOUNT_COUNT") {
        config.account_count = account_count.parse()?;
    }
    
    if let Ok(concurrency) = env::var("CONCURRENCY") {
        config.concurrency = concurrency.parse()?;
    }
    
    if let Ok(transaction_count) = env::var("TRANSACTION_COUNT") {
        config.transaction_count = transaction_count.parse()?;
    }
    
    if let Ok(max_retries) = env::var("MAX_RETRIES") {
        config.max_retries = max_retries.parse()?;
    }
    
    if let Ok(buy_only) = env::var("BUY_ONLY") {
        config.buy_only = buy_only.parse()?;
    }
    
    if let Ok(sell_only) = env::var("SELL_ONLY") {
        config.sell_only = sell_only.parse()?;
    }
    
    if let Ok(market_type) = env::var("MARKET_TYPE") {
        config.market_type_override = match market_type.as_str() {
            "bonding_curve" => Some(MarketType::BondingCurve),
            "uniswap_pool" => Some(MarketType::UniswapPool),
            _ => None,
        };
    }
    
    if let Ok(output_csv) = env::var("OUTPUT_CSV") {
        config.output_csv = output_csv;
    }
    
    if let Ok(log_file) = env::var("LOG_FILE") {
        config.log_file = log_file;
    }
    
    if let Ok(accounts_file) = env::var("ACCOUNTS_FILE") {
        config.accounts_file = accounts_file;
    }
    
    println!("Starting Cult Token simulator with database integration...");
    println!("Database URL: {}", config.database_url);
    println!("Loading accounts from: {}", config.accounts_file);
    
    // Create and run the simulator
    let simulator = CultTokenSimulator::new(config).await?;
    simulator.run().await?;
    
    Ok(())
}