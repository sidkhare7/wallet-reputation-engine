use sqlx::{Pool, Postgres};
use crate::models::Account;

pub async fn load_or_create_account(
    pool: &Pool<Postgres>,
    address: &str,
    slug: Option<String>,
    diamond_hand_probability: Option<u32>,
) -> Result<Account, anyhow::Error> {
    if let Ok(account) = get_account_by_address(pool, address).await {
        return Ok(account);
    }

    let account = Account {
        id: None,
        slug,
        diamond_hand_probability: diamond_hand_probability || 0,
        referrer_id: None,
        total_referrals: Some(0),
        fee_collected: 0,
    };

    let id = insert_account(pool, &account).await?;
    Ok(Account { id: Some(id), ..account })
}

pub async fn get_account_by_address(pool: &Pool<Postgres>, address: &str) -> Result<Account, anyhow::Error> {
    let record = sqlx::query_as!(
        Account,
        r#"
        SELECT * FROM account 
        WHERE id = $1
        "#,
        address
    )
    .fetch_one(pool)
    .await?;
    
    Ok(record)
}

pub async fn insert_account(
    pool: &Pool<Postgres>, 
    address: &str,  // Add address parameter
    account: &Account
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        INSERT INTO account (id, slug, diamond_hand_probability, referrer_id, total_referrals, fee_collected)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        address,
        account.slug,
        account.diamond_hand_probability,
        account.referrer_id,
        account.total_referrals,
        account.fee_collected
    )
    .execute(pool)
    .await?;
    
    Ok(())
}