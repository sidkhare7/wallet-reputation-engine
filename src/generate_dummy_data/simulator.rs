#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();
    
    // Override with environment variables if present
    let rpc_url = env::var("MONAD_TESTNET_RPC_URL") ;
    let database_url = env::var("DATABASE_URL") ;
    let token_address = env::var("CULT_TOKEN_ADDRESS") ;
    let token_id = env::var("CULT_TOKEN_ID") ;
    
    let account_count = env::var("ACCOUNT_COUNT") ;
    
    let concurrency = env::var("CONCURRENCY") ;
    
    let transaction_count = env::var("TRANSACTION_COUNT") ;
    
    let max_retries = env::var("MAX_RETRIES") ;
    
    let buy_only = env::var("BUY_ONLY") ;
    
    let sell_only = env::var("SELL_ONLY") ;
    
    let market_type = env::var("MARKET_TYPE");
    
    let output_csv = env::var("OUTPUT_CSV") ;
    
    let log_file = env::var("LOG_FILE") ;
    
    let accounts_file = env::var("ACCOUNTS_FILE") ;
    
    println!("Starting Cult Token simulator with database integration...");
    println!("Database URL: {}", config.database_url);
    println!("Loading accounts from: {}", config.accounts_file);
    
    // Create and run the simulator
    let simulator = CultTokenSimulator::new(config).await?;
    simulator.run().await?;
    
    Ok(())
}