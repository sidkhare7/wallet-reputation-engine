

use sqlx::{Pool, Postgres};
use tokio_cron_scheduler::{Job, JobScheduler};
use std::sync::Arc;

pub async fn start_token_stats_scheduler(
    pool: Pool<Postgres>
) -> Result<Arc<JobScheduler>, Box<dyn std::error::Error>> {
    let sched = Arc::new(JobScheduler::new().await?);
    let pool = Arc::new(pool);

    // ---- Job definition (8 AM and 8 PM) ----
    // Cron format here is: sec min hour day-of-month month day-of-week
    let cron = "0 0 8,20 * * *";

    
    let pool_for_job = Arc::clone(&pool);

    let job = Job::new_async(cron, move |_uuid, _l| {
        let pool = Arc::clone(&pool_for_job);
        Box::pin(async move {
            log::info!(" Token stats cron fired; starting update…");
            match run_token_stats_task(&pool).await {
                Ok((z_scores, users, duration_ms)) => {
                    log::info!(
                        "Token stats update ok | z_scores={} users={} took={}ms",
                        z_scores, users, duration_ms
                    );
                }
                Err(e) => {
                    log::error!(" Token stats update failed: {e}");
                    sentry::capture_message(
                        &format!("Scheduled token stats update failed: {e}"),
                        sentry::Level::Error,
                    );
                }
            }
        })
    })?;

    sched.add(job).await?;

    // Used for debugging and testing
    // Optional: run once immediately so you can see it works without waiting for the next tick.
    // {
    //     let pool = Arc::clone(&pool);
    //     tokio::spawn(async move {
    //         log::info!(" Running token stats job once at startup for verification…");
    //         if let Err(e) = run_token_stats_task(&pool).await {
    //             log::error!("Startup run failed: {e}");
    //         }
    //     });
    // }

    // Start the scheduler *and keep it alive*
    {
        let sched_for_bg = Arc::clone(&sched);
        tokio::spawn(async move {
            if let Err(e) = sched_for_bg.start().await {
                log::error!("Scheduler start error: {e}");
            }
            futures::future::pending::<()>().await;
        });
    }

    log::info!("Token stats scheduler started; cron={}", cron);
    Ok(sched)
}

async fn run_token_stats_task(pool: &Pool<Postgres>) -> Result<(u64, u64, u64), anyhow::Error> {
    let start = std::time::Instant::now();

    let (z_score_records_updated, users_updated) =
        crate::utils::update_token_stats::sequential_update_all_stats_optimized(
            pool,
            Some(100),   // token limit
            Some(100000), // user limit
        )
        .await?;

    sqlx::query("REFRESH MATERIALIZED VIEW account_reputation_rankings")
        .execute(pool)
        .await?;

    Ok((
        z_score_records_updated,
        users_updated,
        start.elapsed().as_millis() as u64,
    ))
}
