-- SELECT * from diamond_hand_list
-- SELECT encode(merkle_root, 'hex') AS merkle_root_hex, holder_count FROM communities;
-- SELECT * from token_airdrops

-- SELECT * FROM communities;
-- SELECT account.id, diamond_hand_probability, reputation, first_bought, volume, holding_duration, holdings_value, buy_volume, sell_volume, buy_volume_z, sell_volume_z, duration_z, volume_z
-- FROM account
-- JOIN token_balance ON account.id = token_balance.account_id ORDER BY reputation DESC;
-- SELECT * FROM account_reputation_rankings LIMIT 1000;
-- SELECT * FROM account WHERE id = '0xa70C4A15BF088b0C50d28de469239305A35B20db';
-- SELECT * FROM token_balance WHERE account_id = '0x27fAa6497818EC151fb1828D68b60fB6966e4063';
-- SELECT * FROM account WHERE id = '0x7619e2b608E1f5B032C7Efc28dbE9a96d0dBd059';
-- DELETE FROM token_balance WHERE account_id = '0x000000000000000000000000000000000000dEaD';
-- SELECT * FROM token_balance WHERE token_id = '0x93C33B999230eE117863a82889Fdb342cd6D5C64';

-- SELECT user_id from account_reputation_rankings ORDER BY reputation DESC limit 1000;

-- SELECT
--     a.id,
--     a.diamond_hand_probability AS account_diamond_hand_probability,
--     a.reputation AS account_reputation,
--     arr.reputation AS arr_reputation,
--     arr.global_rank,
--     -- For token_id 1
--     MAX(CASE WHEN tb.token_id = '0xAbF39775d23c5B6C0782f3e35B51288bdaf946e2' THEN tb.holdings_value END) AS holdings_value_token_cult,
--     MAX(CASE WHEN tb.token_id = '0xAbF39775d23c5B6C0782f3e35B51288bdaf946e2' THEN tb.holding_duration END) AS holding_duration_token_cult,
--     MAX(CASE WHEN tb.token_id = '0xAbF39775d23c5B6C0782f3e35B51288bdaf946e2' THEN tb.buy_volume END) AS buy_volume_token_cult,
--     -- For token_id 2
--     MAX(CASE WHEN tb.token_id = '0x93C33B999230eE117863a82889Fdb342cd6D5C64' THEN tb.holdings_value END) AS holdings_value_token_gm,
--     MAX(CASE WHEN tb.token_id = '0x93C33B999230eE117863a82889Fdb342cd6D5C64' THEN tb.holding_duration END) AS holding_duration_token_gm,
--     MAX(CASE WHEN tb.token_id = '0x93C33B999230eE117863a82889Fdb342cd6D5C64' THEN tb.buy_volume END) AS buy_volume_token_gm
-- FROM
--     account a
-- LEFT JOIN
--     account_reputation_rankings arr ON a.id = arr.user_id
-- LEFT JOIN
--     token_balance tb ON a.id = tb.account_id
-- GROUP BY
--     a.id, a.diamond_hand_probability, a.reputation, arr.reputation, arr.global_rank
-- ORDER BY
--     arr_reputation DESC NULLS LAST
-- LIMIT 1000;

-- SELECT * FROM token_stats;
-- SELECT * FROM cult_token WHERE id = '0x3b209BaAFEAEeEF3715cDC9dDD44D16A57d428c9';
-- SELECT * FROM account where id = '0x1b2D134Fc716561Afbf8ccEF56568D18c28A34aA';
-- SELECT * FROM token_trade
-- SELECT * FROM cult_token WHERE token_creator = '0x74d8df0c2b9eddfb23a15099bbcd0a35b80baf90';
-- SELECT * from account_community_summary
-- SELECT jsonb_array_elements_text(
--   (merkle_proofs ->> '0x01e24055940879953094Af6cBEc4e58ad4E4421e')::jsonb
-- ) AS proof_element
-- FROM communities
-- WHERE id = '0x9c8ff314c9bc7f6e59a9d9225fb22946427edc03';



--SELECT * from communities where id = '0x000000000000000000000000000000000d1a305d';

-- SELECT 
--     ta.token_id,
--     ta.merkle_root,
--     ta.merkle_proofs,
--     ta.community_id,
--     ta.community_name,
--     ta.total_amount,
--     ta.transaction_hash
-- FROM token_airdrops ta
-- JOIN airdrop_recipients ar ON ta.id = ar.token_airdrop_id
-- WHERE ar.account_id = '0x74d8dF0c2b9EDdFB23a15099BbcD0a35b80BaF90'
--   AND ta.token_id = '0x6662d0981e6E8b4ae30AEcC5141C9EaB3881A6b2'
-- LIMIT 1;

-- SELECT * from token_trade where token_id = '0x3b209BaAFEAEeEF3715cDC9dDD44D16A57d428c9';

-- SELECT * FROM cult_token where id = '0x3b209BaAFEAEeEF3715cDC9dDD44D16A57d428c9';
-- SELECT * FROM cult_token;
-- SELECT * FROM account LIMIT 1;
-- SELECT * FROM communities;
-- SELECT * FROM diamond_hand_list;

-- SELECT 
--     ct.id,
--     ct.name,
--     ct.symbol,
--     ct.token_uri as ipfs_content,
--     COALESCE(tb.holdings_value::TEXT, '0') as user_balance
-- FROM cult_token ct
-- LEFT JOIN token_balance tb 
--     ON ct.id = tb.token_id 
--     AND tb.account_id = '0x74d8df0c2b9eddfb23a15099bbcd0a35b80baf90'
-- WHERE ct.token_creator = '0x74d8df0c2b9eddfb23a15099bbcd0a35b80baf90'
-- ORDER BY ct.block_timestamp DESC;

-- SELECT * FROM cult_token WHERE is_graduated = FALSE;
-- SELECT id FROM communities;
--SELECT * FROM token_airdrops;
-- SELECT * FROM token_trade;
-- SELECT * FROM account LIMIT 1;
-- SELECT * FROM upcoming_tokens;
-- SELECT * FROM account WHERE 'id' = "0x1b2D134Fc716561Afbf8ccEF56568D18c28A34aA";
-- SELECT * FROM cult_token;1
-- SELECT user_id FROM account_reputation_rankings ORDER BY reputation DESC LIMIT 5000;
