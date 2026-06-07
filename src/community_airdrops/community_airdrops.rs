use alloy::primitives::{Address};
use chrono::Utc;
//use reqwest::Error;
use serde::Deserialize;
use std::str::FromStr;
use std::fs::File;
use std::io::BufReader;
use anyhow::{Result, anyhow, Context};
use serde_json::from_reader;
use sqlx::PgPool;
use crate::community_airdrops::handler;
use sqlx::FromRow;
use sqlx::types::BigDecimal;
//use cult_backend::auth::middleware::ApiGuard;
//use cult_backend::config::Settings;
use crate::utils::update_contract_merkle_roots::update_contract_merkle_roots;
use crate::utils::merkle_tree_utils::{create_merkle_tree};

#[derive(Debug, FromRow)] 
pub struct Community {
    pub id: String, 
    pub name: String,    // Unique name for the community
    pub img_url:String,// Unique image for the community
    pub address: String, // Address of the community (unique identifier)
    pub chain: String,   // Blockchain chain (e.g., "Ethereum", "Polygon")
    pub merkle_root: Option<Vec<u8>>, // Merkle root (calculated later)
    pub last_updated_time: Option<chrono::DateTime<Utc>>, // Last updated timestamp
    pub merkle_proofs: Option<serde_json::Value>, // Address -> Merkle proofs
    pub holder_count: i64,
    pub community_score: Option<BigDecimal>,
}
#[derive(Debug, Deserialize)]
pub struct NftOwnersResponse {
    pub owners: Vec<String>,
}

pub fn load_community_configs() -> Result<Vec<CommunityConfig>> {
    let base_path = std::env::current_dir()?;
    let full_path = base_path.join("src").join("community_airdrops").join("communities.json");

    let file = File::open(&full_path)?;
    let reader = BufReader::new(file);
    let configs: Vec<CommunityConfig> = from_reader(reader)?;
    Ok(configs)
}

#[derive(Debug, Clone, Deserialize)]
struct CommunityConfig {
    name: String,
    address: String,
    chain: String,
    img_url:String
}


impl Community {
    pub fn new(name: String, img_url:String, address: String, chain: String) -> Result<Self, String> {
        // Validate the address using Alloy
        if Address::from_str(&address).is_err() {
            return Err("Invalid Ethereum address".to_string());
        }

        Ok(Self {
            id:address.clone(),
            name,
            img_url,
            address,
            chain,
            holder_count:0 as i64,
            merkle_root: None,
            last_updated_time: None,
            merkle_proofs: None,
            community_score:None
        })
    }

    // Method to calculate and set the merkle root and proofs
    pub fn set_merkle_data(&mut self, leaves: Vec<String>) -> Result<(), String> {
        let merkle_data = create_merkle_tree(&leaves)
            .map_err(|e| e.to_string())?;

        self.merkle_root = Some(merkle_data.root);
        self.merkle_proofs = Some(merkle_data.proofs);
        self.last_updated_time = Some(Utc::now());

        Ok(())
    }
}

pub async fn fetch_nft_holders(
    config: &CommunityConfig, 
    api_key: &str
) -> Result<Vec<String>, anyhow::Error> {
    let url = format!(
        "https://{}.g.alchemy.com/nft/v3/{}/getOwnersForContract?contractAddress={}&withTokenBalances=false",
        config.chain, api_key, config.address
    );

    let response = reqwest::get(&url).await?;

    let response: NftOwnersResponse = response.json().await?;
    
    let owners = response.owners;
    Ok(owners)
}
/// Invalidates all old merkle roots in the contract
async fn invalidate_old_roots(pool: &PgPool) -> Result<(), anyhow::Error> {
    println!("Invalidating all existing merkle roots in contract");

    let existing_communities = sqlx::query!(
        r#"SELECT merkle_root FROM communities WHERE merkle_root IS NOT NULL"#
    )
    .fetch_all(pool)
    .await?;

    if existing_communities.is_empty() {
return Ok(());
    }

    let batch: Vec<_> = existing_communities
    .into_iter()
    .map(|c| (c.merkle_root, 0))
    .collect();

    if !batch.is_empty() {
        let (roots, counts): (Vec<_>, Vec<_>) = batch
            .into_iter()
            .map(|(root, _)| (format!("0x{}", hex::encode(root)), 0i64))
            .unzip();

        update_contract_merkle_roots(roots, counts).await?;
    }

    Ok(())
}

async fn update_new_roots(roots_and_counts: Vec<(Vec<u8>, i64)>) -> Result<()> {
    println!("Updating contract with new merkle roots");

    if !roots_and_counts.is_empty() {
        let (roots, counts): (Vec<_>, Vec<_>) = roots_and_counts
            .into_iter()
            .map(|(root, count)| (format!("0x{}", hex::encode(root)), count))
            .unzip();

        update_contract_merkle_roots(roots, counts)
            .await
            .context("Failed to update new merkle roots in contract")?;
    }

    Ok(())
}
pub async fn update_all_communities(pool: &PgPool, api_key: &str) -> Result<(), anyhow::Error> {
    invalidate_old_roots(pool).await?;
    let configs = load_community_configs()
    .context("Failed to load community configs")?;
 
    // Parallelize NFT holder fetching
    let fetch_tasks: Vec<_> = configs.iter()
        .map(|config| fetch_nft_holders(config, api_key))
        .collect();
    
    let holders = futures::future::try_join_all(fetch_tasks).await?;
    
    // Store roots and counts for batch update
    let mut new_roots_and_counts = Vec::new();
    
    for (config, owners) in configs.iter().zip(holders) { 

    //for (config, owners) in configs.into_iter().zip(holders) {
        let mut tx: sqlx::Transaction<'_, sqlx::Postgres> = pool.begin().await?;

        // Fetch existing community with its ID
        let existing = handler::get_community_by_address(&mut tx, &config.address).await?;
        println!("Existing communities");

        // Offload Merkle tree generation to blocking thread
        let merkle_data = {
            let config = config.clone();
            let owners = owners.clone();
            
            tokio::task::spawn_blocking(move || {
                let mut community = Community::new(
                    config.name,
                    config.img_url,
                    config.address,
                    config.chain
                ).map_err(|e| anyhow!("Community creation error: {}", e))?;
                
                community.set_merkle_data(owners)
                    .map_err(|e| anyhow!("Merkle data calculation failed: {}", e))?;
                
                Ok::<_, anyhow::Error>((community.merkle_root, community.merkle_proofs))
            }).await??
        };
        let holder_count = owners.len() as i64;

        if let Some(root) = &merkle_data.0 {
            new_roots_and_counts.push((root.clone(), holder_count));
        }
        println!("CHECKPOINT1");

        let merkle_root = merkle_data.0.as_ref();
        let merkle_proofs = merkle_data.1.as_ref();
        let address = &config.address;
        
        println!(
            "Updating {}: merkle_root len={:?}, merkle_proofs len={:?}",
            address,
            merkle_root.map(|v| v.len()),
            merkle_proofs.map(|v| v.to_string().len())
        );

        // Update or create community in database
        match existing {
            Some(c) => handler::update_community(
                &mut tx,
                &c.address,
                merkle_root,
                merkle_proofs,
                Some(Utc::now()),
                holder_count
            ).await?,
            None => handler::create_community(
                &mut tx,
                &config.name,
                &config.img_url,
                &config.address,
                &config.chain,
                holder_count,
                merkle_root,
                merkle_proofs,
                Some(Utc::now())
            ).await?
        };
        println!("Created community");



        //TODO: this needs to be updated to choose different defaults
        handler::generate_dummy_account_data(
            &mut tx,
            &owners
        ).await?;

        // Atomic membership refresh
        handler::refresh_community_memberships(
            &mut tx,
            config.address.clone(),
            &owners
        ).await?;

        tx.commit().await?;
        println!("Committed transaction for: {}", config.name);
    }
    

       // Update contract with all new roots at once
       update_new_roots(new_roots_and_counts).await?;
       println!("All communities processed");
    Ok(())
}

fn main(){}
// #[tokio::main]
// async fn main() -> std::io::Result<()> {
//     // Load environment variables from .env file
//     dotenv().ok();
//     let settings = Settings::new().expect("Failed to load settings");

//       HttpServer::new(move || {
//         App::new()
//             .wrap(ApiGuard::new())
//             .service(
//                 web::resource("/run-community-airdrops")
//                 .route(web::post().to(|| async {
//                     let result: Result<_, Box<dyn std::error::Error>> = async {
//                         let api_key = env::var("ALCHEMY_API_KEY").expect("ALCHEMY_API_KEY must be set");
//                         let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"); 
//                         let pool = PgPool::connect(&database_url).await?;
                        
//                         update_all_communities(&pool, &api_key).await?;
//                         Ok(())
//                     }.await;

//                     match result {
//                         Ok(_) => HttpResponse::Ok().body("Diamond hands executed"),
//                         Err(e) => HttpResponse::InternalServerError().body(e.to_string())
//                     }
//                 }))
//             )
//     })
//     .bind(&settings.bind_address)?
//     .run()
//     .await
// }