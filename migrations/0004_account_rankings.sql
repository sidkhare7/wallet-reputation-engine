CREATE MATERIALIZED VIEW account_reputation_rankings AS
SELECT
    id AS user_id,
    reputation,
    RANK() OVER (ORDER BY reputation DESC) AS global_rank
FROM account
WHERE slug IS NULL AND reputation > 0;