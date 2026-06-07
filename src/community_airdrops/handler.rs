use sqlx::{PgPool};
use chrono::prelude::*;
use anyhow::Result;
use super::community_airdrops::Community;
use sqlx::postgres::Postgres;
use rand::prelude::*;
//use crate::utilities::account::load_or_create_account;

pub async fn create_community(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    name: &str,
    img_url: &str,
    address: &str,
    chain: &str,
    holder_count:i64,
    merkle_root: Option<&Vec<u8>>,
    merkle_proofs: Option<&serde_json::Value>,
    last_updated: Option<DateTime<Utc>>,
    
) -> Result<Community> {
    println!("creating community");
    let community = sqlx::query_as!(
        Community,
        r#"
        INSERT INTO communities (id, name, img_url, address, chain, merkle_root, merkle_proofs, last_updated_time, holder_count)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                           RETURNING id, name, img_url, address, chain, merkle_root, merkle_proofs, last_updated_time, holder_count, 
                community_score as "community_score: _"
        "#,
        address,
        name,
        img_url,
        address,
        chain,
        merkle_root,
        merkle_proofs,
        last_updated,
        holder_count
    )
    .fetch_one(&mut **tx)
    .await?;
println!("created community");
    Ok(community)
}

pub async fn update_community(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    address: &str,
    merkle_root: Option<&Vec<u8>>,
    merkle_proofs: Option<&serde_json::Value>,
    last_updated: Option<DateTime<Utc>>,
    holder_count:i64
) -> Result<Community> {
    println!("updating community");
    let community = sqlx::query_as!(
        Community,
        r#"
        UPDATE communities
        SET 
            merkle_root = COALESCE($1, merkle_root),
            merkle_proofs = COALESCE($2, merkle_proofs),
            last_updated_time = $3,
            holder_count = $5
        WHERE address = $4
                             RETURNING id, name, img_url, address, chain, merkle_root, merkle_proofs, last_updated_time, holder_count,
                community_score as "community_score: _"
        "#,
        merkle_root,
        merkle_proofs,
        last_updated,
        address,
        holder_count
    )
    .fetch_one(&mut **tx)
    .await?;
    
    println!("updated community");
    Ok(community)
}



pub async fn get_community_by_address(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    address: &str
) -> Result<Option<Community>, sqlx::Error> {
    let community = sqlx::query_as::<_, Community>(
        r#"SELECT * FROM communities WHERE address = $1"#,
    )
    .bind(address)
    .fetch_optional(&mut **tx) // Dereference the transaction
    .await?;

    Ok(community)
}

pub async fn refresh_community_memberships<'a>(
    tx: &'a mut sqlx::Transaction<'_, Postgres>,
    community_id: String,
    new_members: &[String],
) -> Result<(), sqlx::Error> {
    // Clear existing memberships
    sqlx::query!(
        "DELETE FROM account_communities WHERE community_id = $1",
        community_id.clone()
    )
    .execute(&mut **tx)
    .await?;

    println!("Trying to put community ID - {}", community_id);

    // Batch insert new memberships using UNNEST
    if !new_members.is_empty() {
        for chunk in new_members.chunks(100) {
            sqlx::query!(
                r#"
                INSERT INTO account_communities (account_id, community_id)
                SELECT a.id, $1
                FROM UNNEST($2::text[]) AS m(id)
                JOIN account a ON a.id = m.id
                ON CONFLICT (account_id, community_id) DO NOTHING
                "#,
                &community_id,
                chunk as &[String]
            )
            .execute(&mut **tx)
            .await?;
        }
    }

    println!("Refreshed Community");

    Ok(())
}

pub async fn generate_dummy_account_data<'a>(
    tx: &'a mut sqlx::Transaction<'_, Postgres>,
    addresses: &[String],
) -> Result<(), sqlx::Error> {
    //let mut rng = rand::thread_rng();

    println!("Addresses: {:?}", addresses.len());
    // Bulk check existing accounts
    let existing_accounts: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM account WHERE id = ANY($1)"
    )
    .bind(addresses)
    .fetch_all(&mut **tx)
    .await?;

    println!("Checked existing accounts");

    // Identify missing accounts and bulk insert
    let missing: Vec<String> = addresses.iter()
        .filter(|addr| !existing_accounts.contains(addr))
        .cloned()
        .collect();

    if !missing.is_empty() {
        sqlx::query(
            r#"
            INSERT INTO account (
                id, 
                diamond_hand_probability, 
                fee_collected,
                total_referrals
            ) 
            SELECT 
                unnest($1::text[]),
                100,  -- default diamond_hand_probability
                0,  -- default fee_collected
                0   -- default total_referrals
            "#
        )
        .bind(&missing)
        .execute(&mut **tx)
        .await?;
    }
    println!("Existing Accounts: {:?}", existing_accounts.len());
    println!("Missing accounts: {}", missing.len());

    // Generate random data in memory
    // let mut updates = Vec::with_capacity(addresses.len());
    // for address in addresses {
    //     let probability = rng.gen_range(0..=500);
    //     let fee_collected = rng.gen_range(0..1_000_000);
        
    //     let referrer_id = if addresses.len() > 1 && rng.gen_bool(0.5) {
    //         addresses.choose(&mut rng)
    //             .filter(|&a| a != address)
    //             .cloned()
    //     } else {
    //         None
    //     };

    //     updates.push((
    //         address,
    //         probability,
    //         fee_collected,
    //         referrer_id,
    //         format!("REF-{:06}", rng.gen::<u32>() % 1000000),
    //         format!("twitter_user_{}", rng.gen::<u32>()),
    //         format!("discord_user_{}", rng.gen::<u32>()),
    //     ));
    // }

    // // Bulk update using UNNEST
    // let (ids, probs, fees, referrers, codes, twitters, discords) = updates
    //     .into_iter()
    //     .fold(
    //         (vec![], vec![], vec![], vec![], vec![], vec![], vec![]),
    //         |mut acc, row| {
    //             acc.0.push(row.0);
    //             acc.1.push(row.1);
    //             acc.2.push(row.2);
    //             acc.3.push(row.3);
    //             acc.4.push(row.4);
    //             acc.5.push(row.5);
    //             acc.6.push(row.6);
    //             acc
    //         },
    //     );

    // sqlx::query(
    //     r#"
    //     UPDATE account AS a SET
    //         diamond_hand_probability = data.probability,
    //         fee_collected = data.fee,
    //         referrer_id = data.referrer,
    //         referral_code = data.code,
    //         twitter = data.twitter,
    //         discord = data.discord
    //     FROM (
    //         SELECT
    //             unnest($1::text[]) as id,
    //             unnest($2::int[]) as probability,
    //             unnest($3::bigint[]) as fee,
    //             unnest($4::text[]) as referrer,
    //             unnest($5::text[]) as code,
    //             unnest($6::text[]) as twitter,
    //             unnest($7::text[]) as discord
    //     ) AS data
    //     WHERE a.id = data.id
    //     "#
    // )
    // .bind(ids)
    // .bind(probs)
    // .bind(fees)
    // .bind(referrers)
    // .bind(codes)
    // .bind(twitters)
    // .bind(discords)
    // .execute(&mut **tx)
    // .await?;

    // // Bulk update all referral counts in one query
    // sqlx::query(
    //     r#"
    //     UPDATE account
    //     SET total_referrals = (
    //         SELECT COUNT(*) 
    //         FROM account AS referrers 
    //         WHERE referrers.referrer_id = account.id
    //     )
    //     "#
    // )
    // .execute(&mut **tx)
    // .await?;

    Ok(())
}


pub async fn delete_community(pool: &PgPool, address: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // First delete memberships
    sqlx::query!(
        r#"
        DELETE FROM account_communities
        WHERE community_id = (
            SELECT id FROM communities WHERE address = $1
        )
        "#,
        address
    )
    .execute(&mut *tx)
    .await?;

    // Then delete the community
    sqlx::query!(
        r#"
        DELETE FROM communities
        WHERE address = $1
        "#,
        address
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}


// pub async fn fetchAccountNFTs(network: &str, user_address: i64) -> Result<(), anyhow::Error> {
//     let url = format!(
//         "https://{}.g.alchemy.com/nft/v3/{}/getContractsForOwner?owner={}&pageSize=100&withMetadata=false",
//         network,
//         env::var("ALCHEMY_API_KEY")?,
//         user_address
//     );

//     let response = reqwest::get(&url).await?;

//     if response.status().is_success() {
//         let  = response.json().await?;
//         println!("Updated NFT Holders for Account {}:", user_address);
//     } else {
//         println!("Failed to fetch NFTs for {}: {}", user_address, response.status());
//     }
//     Ok(())
// }