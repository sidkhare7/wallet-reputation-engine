use alloy::primitives::Address;
use alloy_provider::{Provider, ProviderBuilder};
use alloy::network::TransactionResponse;
use anyhow::Result;
use log::error;
use std::collections::HashMap;
use std::env;
use std::str::FromStr;

// Remove the complex type alias and just use provider creation functions

// Hardcoded mapping of token_id to (airdrop_contract, pool_contract)
lazy_static::lazy_static! {
    pub static ref TOKEN_CONTRACTS: HashMap<&'static str, (&'static str, &'static str)> = {
        let mut map = HashMap::new();
        map.insert("0xAbF39775d23c5B6C0782f3e35B51288bdaf946e2", ("0xddd77e91C222e36d4bd5Ca8E78682a2172314197", "0x9D950CeBb5118756b13BA635d0d853494ED9c950"));
        map.insert("0x93C33B999230eE117863a82889Fdb342cd6D5C64", ("0xAD768d53CD6DD556a594EF78c020935Ec838C2A9", "0x7C2b946FAb9207ed045bC7702E0f72718bA28280"));
        map.insert("0x69c29fe56771f295a73Af25cc70edf58aF5954d5", ("0x3E8a58721dA5E1F83f14075442d0dc4869B24a4D", "0xD322Db88906cD68265b41EDb3371BA6a48C498D9"));
        map.insert("0xda09E49028ff33939bf9E213EEAa3F723acfa218", ("0x8bD72aBDc03b96302621D240cD1B787E24c5F128", "0xc41faa3eE4815651bf6D0a02266682fddaeB0d0b"));
        map
    };
}

/// Helper function to convert string to checksum address
pub fn to_checksum_address(address_str: &str) -> Result<String, anyhow::Error> {    
    // Parse the address using alloy_primitives::Address which handles valid validation and checksum
    let address = Address::from_str(address_str)
        .map_err(|e| anyhow::anyhow!("Invalid address '{}': {}", address_str, e))?;
    
    // Return the checksummed address
    Ok(address.to_string())
}

/// Helper function to print new token contracts (for manual addition to the constant mapping)
/// This replaces the old register_token_contracts function
pub fn print_token_contracts(token_id: &str, airdrop_contract: &str, pool_contract: &str) {
    println!("==========================================");
    println!("NEW TOKEN CONTRACT CREATED");
    println!("==========================================");
    println!("Token ID:         {}", token_id);
    println!("Airdrop Contract: {}", airdrop_contract);
    println!("Pool Contract:    {}", pool_contract);
    println!("==========================================");
    println!("To add this to the hardcoded mapping, add this line to TOKEN_CONTRACTS in src/utils/misc.rs:");
    println!("map.insert(\"{}\", (\"{}\", \"{}\"));", token_id, airdrop_contract, pool_contract);
    println!("==========================================");
}

/// Helper function to get contracts for a token safely
pub fn get_token_contracts(token_id: &str) -> Option<(String, String)> {
    TOKEN_CONTRACTS
        .get(token_id)
        .map(|(airdrop, pool)| (airdrop.to_string(), pool.to_string()))
}

/// Helper function to list all registered tokens (for debugging)
pub fn list_registered_tokens() -> Vec<(String, String, String)> {
    TOKEN_CONTRACTS
        .iter()
        .map(|(token_id, (airdrop, pool))| {
            (token_id.to_string(), airdrop.to_string(), pool.to_string())
        })
        .collect()
}

/// Helper function to get the count of registered tokens
pub fn get_registered_tokens_count() -> usize {
    TOKEN_CONTRACTS.len()
}

/// Helper function to get token_id by pool address
pub fn get_token_id_by_pool_address(pool_address: &str) -> Option<String> {
    TOKEN_CONTRACTS
        .iter()
        .find(|(_, (_, pool))| *pool == pool_address)
        .map(|(token_id, _)| token_id.to_string())
}

/// Helper function to check if an address is a known contract
pub fn is_known_contract(address: &str) -> bool {
    TOKEN_CONTRACTS
        .values()
        .any(|(airdrop, pool)| *airdrop == address || *pool == address)
}

/// Check if an address is a contract by fetching its bytecode via RPC
pub async fn is_contract_address_fallback(address: &str) -> Result<bool> {
    // Check if it's a known contract from our registry first
    if is_known_contract(address) {
        return Ok(true);
    }

    // Check if it's a zero address (which is not a contract)
    if address == "0x0000000000000000000000000000000000000000" {
        return Ok(false);
    }

    // Try to get the RPC URL from environment
    let rpc_url = match env::var("MONAD_TESTNET_RPC_URL") {
        Ok(url) if !url.is_empty() => url,
        _ => {
            error!("MONAD_TESTNET_RPC_URL not set, falling back to known contracts only");
            return Ok(false);
        }
    };

    // Parse the address
    let address = match Address::from_str(address) {
        Ok(addr) => addr,
        Err(_) => {
            error!("Invalid address format: {}", address);
            return Ok(false);
        }
    };

    // Create provider and fetch bytecode
    let provider = ProviderBuilder::new().on_http(rpc_url.parse().unwrap());
    match provider.get_code_at(address).await {
        Ok(code) => {
            // If bytecode is not empty, it's a contract
            Ok(!code.0.is_empty())
        }
        Err(e) => {
            error!("Failed to fetch bytecode for address {}: {}", address, e);
            // On error, assume it's not a contract to be safe
            Ok(false)
        }
    }
}

/// Get the 'from' address of a transaction by its hash
pub async fn get_transaction_from_address_fallback(tx_hash: &str) -> Result<Option<String>> {
    // Try to get the RPC URL from environment
    let rpc_url = match env::var("MONAD_TESTNET_RPC_URL") {
        Ok(url) if !url.is_empty() => url,
        _ => {
            error!("MONAD_TESTNET_RPC_URL not set, cannot fetch transaction details");
            return Ok(None);
        }
    };

    // Parse the transaction hash
    let tx_hash = match alloy::primitives::FixedBytes::from_str(tx_hash) {
        Ok(hash) => hash,
        Err(_) => {
            error!("Invalid transaction hash format: {}", tx_hash);
            return Ok(None);
        }
    };

    // Create provider and fetch transaction details
    let provider = ProviderBuilder::new().on_http(rpc_url.parse().unwrap());
    match provider.get_transaction_by_hash(tx_hash).await {
        Ok(Some(tx)) => {
            let from_address = tx.from().to_string();
            println!("Successfully fetched tx.from from blockchain: {}", from_address);
            Ok(Some(from_address))
        }
        Ok(None) => {
            error!("Transaction not found: {:?}", tx_hash);
            Ok(None)
        }
        Err(e) => {
            error!("Failed to fetch transaction details for {:?}: {}", tx_hash, e);
            Ok(None)
        }
    }
}

    
pub async fn get_token_balance_from_contract(
    token_id: &str,
    account_address: &str,
) -> Result<Option<sqlx::types::BigDecimal>> {
    // Get RPC URL from environment
    let rpc_url = match env::var("MONAD_TESTNET_RPC_URL") {
        Ok(url) if !url.is_empty() => url,
        _ => {
            error!("MONAD_TESTNET_RPC_URL not set, cannot fetch token balance");
            return Ok(None);
        }
    };

    // Parse addresses with error handling
    let parse_address = |s: &str, desc: &str| match Address::from_str(s) {
        Ok(addr) => Some(addr),
        Err(_) => {
            error!("Invalid {} address format: {}", desc, s);
            None
        }
    };

    let Some(token_address) = parse_address(token_id, "token") else {
        return Ok(None);
    };
    let Some(account_addr) = parse_address(account_address, "account") else {
        return Ok(None);
    };

    // Create provider
    let url = match rpc_url.parse() {
        Ok(url) => url,
        Err(e) => {
            error!("Invalid RPC URL: {}", e);
            return Ok(None);
        }
    };
    let provider = ProviderBuilder::new().on_http(url);

    // Create a simple ERC20 ABI for balanceOf function
    let erc20_abi = serde_json::json!([
        {
            "constant": true,
            "inputs": [{"name": "account", "type": "address"}],
            "name": "balanceOf",
            "outputs": [{"name": "", "type": "uint256"}],
            "payable": false,
            "stateMutability": "view",
            "type": "function"
        }
    ]);

    // Parse JSON into JsonAbi
    let json_abi: alloy::json_abi::JsonAbi = serde_json::from_value(erc20_abi)?;

    // Create contract instance
    let contract = alloy::contract::ContractInstance::new(
        token_address,
        provider,
        alloy::contract::Interface::new(json_abi)
    );

    // Call balanceOf function
    match contract.function("balanceOf", &[alloy::dyn_abi::DynSolValue::Address(account_addr)]) {
        Ok(call) => {
            match call.call().await {
                Ok(result) => {
                    // Decode the result
                    if let Some(alloy::dyn_abi::DynSolValue::Uint(balance, _)) = result.first() {
                        sqlx::types::BigDecimal::from_str(&balance.to_string())
                            .map(|balance_bd| {
                                println!("Successfully fetched token balance from contract: {} for account: {}", 
                                    balance_bd, account_address);
                                Some(balance_bd)
                            })
                            .map_err(|e| {
                                error!("Failed to parse balance to BigDecimal: {}", e);
                                e
                            })
                            .map(Ok)
                            .unwrap_or_else(|_| Ok(None))
                    } else {
                        error!("Unexpected result type from balanceOf call");
                        Ok(None)
                    }
                }
                Err(e) => {
                    error!("Failed to call balanceOf on token {} for account {}: {}", 
                        token_id, account_address, e);
                    Ok(None)
                }
            }
        }
        Err(e) => {
            error!("Failed to create balanceOf function call: {}", e);
            Ok(None)
        }
    }
}