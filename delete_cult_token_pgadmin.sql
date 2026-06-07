-- Delete Cult Token and All Associated Records (pgAdmin Version)
-- This script safely removes a cult_token and all related data
-- Token ID: 0x86E98B858C03214642C50958047912dD5BCB2EFf

-- Start transaction for safety
BEGIN;

-- 1. Delete airdrop_recipients first (references token_airdrops)
DELETE FROM airdrop_recipients 
WHERE token_airdrop_id IN (
    SELECT id FROM token_airdrops WHERE token_id = '0x86E98B858C03214642C50958047912dD5BCB2EFf'
);

-- 2. Delete token_airdrops (references cult_token)
DELETE FROM token_airdrops 
WHERE token_id = '0x86E98B858C03214642C50958047912dD5BCB2EFf';

-- 3. Delete token_balance (references cult_token)
DELETE FROM token_balance 
WHERE token_id = '0x86E98B858C03214642C50958047912dD5BCB2EFf';

-- 4. Delete token_stats (references cult_token)
DELETE FROM token_stats 
WHERE token_id = '0x86E98B858C03214642C50958047912dD5BCB2EFf';

-- 5. Finally delete the cult_token
-- This will CASCADE delete:
-- - token_trade (ON DELETE CASCADE)
-- - token_ohlcv (ON DELETE CASCADE) 
-- - account_watchlist (ON DELETE CASCADE)
DELETE FROM cult_token 
WHERE id = '0x86E98B858C03214642C50958047912dD5BCB2EFf';

-- Show what was deleted
SELECT 
    CASE 
        WHEN EXISTS(SELECT 1 FROM cult_token WHERE id = '0x86E98B858C03214642C50958047912dD5BCB2EFf') 
        THEN 'Token still exists - deletion may have failed'
        ELSE 'Token and all associated records successfully deleted'
    END as result;

-- Commit the transaction
COMMIT; 