-- if previous transaction is aborted
--ROLLBACK;

-- Start fresh
BEGIN;

-- 0. Prepare list of tokens that actually exist in cult_token
CREATE TEMP TABLE tmp_existing_tokens (id text) ON COMMIT DROP;

INSERT INTO tmp_existing_tokens (id)
SELECT t.token_id
FROM (VALUES
  ('0x93C33B999230eE117863a82889Fdb342cd6D5C64')
) AS t(token_id)
JOIN cult_token ct ON ct.id = t.token_id;  -- only insert ones that actually exist

-- 1. Delete dependent airdrop_recipients via token_airdrops
DELETE FROM airdrop_recipients ar
USING token_airdrops ta, tmp_existing_tokens et
WHERE ar.token_airdrop_id = ta.id
  AND ta.token_id = et.id;

-- 2. Delete token_airdrops
DELETE FROM token_airdrops
WHERE token_id IN (SELECT id FROM tmp_existing_tokens);

-- 3. Delete token_balance
DELETE FROM token_balance
WHERE token_id IN (SELECT id FROM tmp_existing_tokens);

-- 4. Delete token_stats
DELETE FROM token_stats
WHERE token_id IN (SELECT id FROM tmp_existing_tokens);

-- 5. Delete cult_tokens (cascades if configured)
-- DELETE FROM cult_token
-- WHERE id IN (SELECT id FROM tmp_existing_tokens);

-- Verification summary
-- SELECT
--   et.id AS token_id,
--   CASE
--     WHEN EXISTS (SELECT 1 FROM cult_token ct WHERE ct.id = et.id) THEN 'FAILED'
--     ELSE 'DELETED'
--   END AS status
-- FROM tmp_existing_tokens et;

COMMIT;
