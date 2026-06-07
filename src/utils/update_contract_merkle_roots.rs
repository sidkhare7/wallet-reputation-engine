use alloy::{
    contract::{ContractInstance, Interface},
    network::EthereumWallet,
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
    primitives::{Address, B256, U256},
    dyn_abi::DynSolValue
};
use std::{env, str::FromStr};
pub async fn update_contract_merkle_roots(
    merkle_roots: Vec<String>,
    holder_counts: Vec<i64>,
) -> Result<String, anyhow::Error>  {
    // Validate input lengths match
    if merkle_roots.len() != holder_counts.len() {
        return Err(anyhow::anyhow!("Input vectors must have equal length"));
    }

    // Parse contract address
    let contract_address = Address::from_str("0x23D58F26Fc4fA9520297b9F66Ba8097006f01E4F")?;

    // Get environment variables
    let private_key = env::var("PRIVATE_KEY")?;
    let rpc_url = env::var("MONAD_TESTNET_RPC_URL")?;

    // Create signer and provider
    let signer: PrivateKeySigner = private_key.parse()?;
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new().wallet(wallet).on_http(rpc_url.parse()?);


        // Load contract ABI from file
    // Get the contract ABI.
    let path = std::env::current_dir()?.join("contracts/abis/CultFactory.json");
    
    

    // Read the artifact which contains `abi`, `bytecode`, `deployedBytecode` and `metadata`.
    let artifact = std::fs::read(path).expect("Failed to read artifact");
    let json: serde_json::Value = serde_json::from_slice(&artifact)?;

    // Get `abi` from the artifact.
    let abi_value = json.get("abi").expect("Failed to get ABI from artifact");
    let abi = serde_json::from_str(&abi_value.to_string())?;

    // Create contract instance
    let contract = ContractInstance::new(
        contract_address,
        provider.clone(),
        Interface::new(abi)
    );

    // Prepare function parameters
    let roots: Vec<DynSolValue> = merkle_roots
        .iter()
        .map(|r| {
            B256::from_str(r)
                .map(|b| DynSolValue::FixedBytes(b.0.into(), 32))
                .map_err(|e| anyhow::anyhow!("Invalid merkle root format: {}", e))
        })
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

    let counts: Vec<DynSolValue> = holder_counts
        .into_iter()
        .map(|c| DynSolValue::Uint(U256::from(c), 32))
        .collect();

    println!("Roots {:?}",roots);
    println!("counts {:?}",counts);

    // Execute contract function
    let tx_hash = contract
        .function("updateMerkleRoots", &[DynSolValue::Array(roots), DynSolValue::Array(counts)])?
        .send()
        .await?;
    //add below to wait for transaction confirmation commenting it saves 16 seconds or 8 sec per call
        //.watch()
        //.await?;

   Ok(format!("{:?}", tx_hash))

}


// // Example usage
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
//     // Load environment variables from .env file if present
//     dotenv::dotenv().ok();
//     // Example values - replace with actual values
//     let merkle_roots = vec![
//         "0x1234567890123456789012345678901234567890123456789012345678901234".to_string(),
//         "0x5678901234567890123456789012345678901234567890123456789012345678".to_string(),
//     ];
    
//     let holder_counts = vec![100, 200];
    
//     match update_contract_merkle_roots(merkle_roots, holder_counts).await {
//         Ok(tx_hash) => println!("Transaction successful! Hash: {}", tx_hash),
//         Err(e) => eprintln!("Error: {}", e),
//     }
    
//     Ok(())
// }
