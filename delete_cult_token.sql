-- Delete Cult Token and All Associated Records (pgAdmin Version)
-- This script safely removes a cult_token and all related data
-- Usage: Replace 'YOUR_TOKEN_ID_HERE' with the actual token ID you want to delete

-- CHANGE THIS: Replace 'YOUR_TOKEN_ID_HERE' with your actual token ID
-- Example: 'YOUR_TOKEN_ID_HERE' -> '0x1234567890abcdef1234567890abcdef12345678'

-- Start transaction for safety
BEGIN;

-- 1. Delete airdrop_recipients first (references token_airdrops)
DELETE FROM airdrop_recipients 
WHERE token_airdrop_id IN (
    SELECT id FROM token_airdrops WHERE token_id = 'YOUR_TOKEN_ID_HERE'
);

-- 2. Delete token_airdrops (references cult_token)
DELETE FROM token_airdrops 
WHERE token_id = 'YOUR_TOKEN_ID_HERE';

-- 3. Delete token_balance (references cult_token)
DELETE FROM token_balance 
WHERE token_id = 'YOUR_TOKEN_ID_HERE';

-- 4. Delete token_stats (references cult_token)
DELETE FROM token_stats 
WHERE token_id = 'YOUR_TOKEN_ID_HERE';

-- 5. Finally delete the cult_token
-- This will CASCADE delete:
-- - token_trade (ON DELETE CASCADE)
-- - token_ohlcv (ON DELETE CASCADE) 
-- - account_watchlist (ON DELETE CASCADE)
DELETE FROM cult_token 
WHERE id = 'YOUR_TOKEN_ID_HERE';

-- Show what was deleted
SELECT 
    CASE 
        WHEN EXISTS(SELECT 1 FROM cult_token WHERE id = 'YOUR_TOKEN_ID_HERE') 
        THEN 'Token still exists - deletion may have failed'
        ELSE 'Token and all associated records successfully deleted'
    END as result;

-- Commit the transaction
COMMIT;

-- Alternative version without psql variables (if not using psql):
/*
-- Replace 'YOUR_TOKEN_ID' with the actual token ID
BEGIN;

DELETE FROM airdrop_recipients 
WHERE token_airdrop_id IN (
    SELECT id FROM token_airdrops WHERE token_id = 'YOUR_TOKEN_ID'
);

DELETE FROM token_airdrops 
WHERE token_id = 'YOUR_TOKEN_ID';

DELETE FROM token_balance 
WHERE token_id = 'YOUR_TOKEN_ID';

DELETE FROM token_stats 
WHERE token_id = 'YOUR_TOKEN_ID';

DELETE FROM cult_token 
WHERE id = 'YOUR_TOKEN_ID';

COMMIT;
*/ 