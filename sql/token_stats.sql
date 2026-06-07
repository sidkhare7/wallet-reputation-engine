SELECT 
  token_id,
  COUNT(DISTINCT account_id) AS total_accounts
FROM token_balance
GROUP BY token_id
ORDER BY total_accounts DESC;

SELECT * FROM account_reputation_rankings;

REFRESH MATERIALIZED VIEW account_reputation_rankings;

SELECT * FROM token_stats;
SELECT * FROM cult_token;

SELECT * FROM token_balance WHERE account_id = '0x63c0c9328d1d9aa15B37Fe300102D5c59256173E';

SELECT tb.*, a.reputation
FROM token_balance tb
JOIN account a ON tb.account_id = a.id
WHERE tb.account_id IN (
  '0x23B557c50B33c55a53b3CD2e18BB3CAaD3BDEA1E', 
  '0x74d8dF0c2b9EDdFB23a15099BbcD0a35b80BaF90',
  '0x963150c743139434cd5D543563800959705ac46f',
  '0xA88f8698A43F3D529f70E98AE96D06b518dA93F8',
  '0x335f5aF787199e6549C93d75FEdff56B8225e25F',
  '0x9613eca44F5e6b3045284537a6Ba4b56BA0E7CA8',
  '0xE523875E36EFD18A984CbAda6C2382Fa3aB2eF67'
);


-- SELECT * FROM token_balance WHERE token_id = '0x93C33B999230eE117863a82889Fdb342cd6D5C64'

-- SELECT * FROM account WHERE "id" = '0x74d8dF0c2b9EDdFB23a15099BbcD0a35b80BaF90'










SELECT tb.*, a.id AS original_address
FROM token_balance tb
JOIN account a ON tb.account_id = a.id
WHERE LOWER(a.id) IN (
  SELECT LOWER(id)
  FROM account
  GROUP BY LOWER(id)
  HAVING COUNT(*) > 1
);



-- Script to remove duplicate token_balance records
-- Keeps only the record with the latest last_updated timestamp for each account_id and token_id combination

-- First, let's identify duplicates to understand the scope
-- SELECT 
--     account_id,
--     token_id,
--     COUNT(*) as duplicate_count,
--     STRING_AGG(last_updated::text, ', ' ORDER BY last_updated DESC) as all_timestamps
-- FROM token_balance 
-- GROUP BY account_id, token_id 
-- HAVING COUNT(*) > 1
-- ORDER BY duplicate_count DESC;

-- -- Show detailed information about duplicates
-- SELECT 
--     account_id,
--     token_id,
--     first_bought,
--     last_updated,
--     volume,
--     pnl,
--     ROW_NUMBER() OVER (PARTITION BY account_id, token_id ORDER BY last_updated DESC) as rn
-- FROM token_balance 
-- WHERE (account_id, token_id) IN (
--     SELECT account_id, token_id 
--     FROM token_balance 
--     GROUP BY account_id, token_id 
--     HAVING COUNT(*) > 1
-- )
-- ORDER BY account_id, token_id, last_updated DESC;

-- -- Create a backup table before deletion (optional but recommended)
-- CREATE TABLE token_balance_backup_before_dedup AS 
-- SELECT * FROM token_balance 
-- WHERE (account_id, token_id) IN (
--     SELECT account_id, token_id 
--     FROM token_balance 
--     GROUP BY account_id, token_id 
--     HAVING COUNT(*) > 1
-- );

-- -- Method 1: Using CTE to delete duplicates (PostgreSQL specific)
-- -- This keeps the record with the latest last_updated timestamp
-- WITH duplicate_records AS (
--     SELECT 
--         account_id,
--         token_id,
--         last_updated,
--         ROW_NUMBER() OVER (PARTITION BY account_id, token_id ORDER BY last_updated DESC) as rn
--     FROM token_balance
-- )
-- DELETE FROM token_balance 
-- WHERE (account_id, token_id, last_updated) IN (
--     SELECT account_id, token_id, last_updated 
--     FROM duplicate_records 
--     WHERE rn > 1
-- );

-- -- Alternative Method 2: Using a temporary table approach
-- -- Uncomment this section if Method 1 doesn't work in your environment

-- /*
-- -- Create temporary table with unique records (latest last_updated only)
-- CREATE TEMP TABLE token_balance_unique AS
-- SELECT DISTINCT ON (account_id, token_id) *
-- FROM token_balance
-- ORDER BY account_id, token_id, last_updated DESC;

-- -- Delete all records from original table
-- DELETE FROM token_balance;

-- -- Insert unique records back
-- INSERT INTO token_balance 
-- SELECT * FROM token_balance_unique;

-- -- Drop temporary table
-- DROP TABLE token_balance_unique;
-- */

-- -- Verify the deduplication worked
-- SELECT 
--     account_id,
--     token_id,
--     COUNT(*) as record_count
-- FROM token_balance 
-- GROUP BY account_id, token_id 
-- HAVING COUNT(*) > 1;

-- -- If the above query returns no rows, deduplication was successful

-- -- Optional: Drop the backup table if you're satisfied with the results
-- -- DROP TABLE token_balance_backup_before_dedup;

-- -- Final verification: Check total record count
-- SELECT 
--     COUNT(*) as total_records,
--     COUNT(DISTINCT (account_id, token_id)) as unique_combinations
-- FROM token_balance;

-- -- The two counts should be equal after successful deduplication 