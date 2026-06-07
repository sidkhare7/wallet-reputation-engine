SELECT 1 FROM airdrop_recipients 
WHERE account_id = $1 AND token_airdrop_id = $2
LIMIT 1;


SELECT ta.* FROM token_airdrops ta
JOIN airdrop_recipients ar ON ta.id = ar.token_airdrop_id
WHERE ar.account_id = $1;