-- Add migration script here
DROP MATERIALIZED VIEW IF EXISTS global_rankings;

CREATE MATERIALIZED VIEW global_rankings AS
WITH user_scores AS (
  SELECT
      a.id AS user_id,
      SUM(tb.duration_z)                           AS duration_score,
      SUM(tb.pnl_z   * ts.holder_weight)           AS pnl_score,
      SUM(tb.value_z * ts.liquidity_score)         AS holdings_score
  FROM token_balance tb
  JOIN token_stats ts ON ts.token_id = tb.token_id
  JOIN account a      ON a.id        = tb.account_id
  WHERE tb.holdings_value > 0
  GROUP BY a.id
),
scored AS (
  SELECT
      user_id,
      duration_score,
      pnl_score,
      holdings_score,
      (0.35 * duration_score
     + 0.25 * pnl_score
     + 0.40 * holdings_score) AS total_score
  FROM user_scores
)
SELECT
    user_id,
    duration_score,
    pnl_score,
    holdings_score,
    total_score,
    ROW_NUMBER() OVER (
      ORDER BY
        total_score   DESC,
        holdings_score DESC,   -- secondary tiebreakers
        pnl_score      DESC,
        duration_score DESC,
        user_id        ASC     -- final tiebreaker
    ) AS global_rank
FROM scored;

CREATE UNIQUE INDEX ON global_rankings (user_id);
