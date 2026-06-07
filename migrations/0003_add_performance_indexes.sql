-- Performance indexes for token stats operations
-- These indexes will dramatically speed up the batch operations

-- Index for token_balance queries by token_id (used in stats calculation)
CREATE INDEX IF NOT EXISTS idx_token_balance_token_id_volume 
ON token_balance(token_id) WHERE volume > 0;

-- Index for filtering by holdings_value in reputation calculations
CREATE INDEX IF NOT EXISTS idx_token_balance_account_holdings 
ON token_balance(account_id) WHERE holdings_value > 0;

-- Composite index for token_balance filtering conditions
CREATE INDEX IF NOT EXISTS idx_token_balance_duration_filter 
ON token_balance(token_id, volume) 
WHERE (holding_duration > 600 OR holding_duration IS NULL);

-- Index for account reputation ranking
CREATE INDEX IF NOT EXISTS idx_account_reputation_rank 
ON account(reputation DESC) WHERE slug IS NULL AND reputation > 0;

-- Index for account diamond hand probability
CREATE INDEX IF NOT EXISTS idx_account_diamond_hand 
ON account(diamond_hand_probability DESC) WHERE slug IS NULL;

-- Composite index for token_stats lookups
CREATE INDEX IF NOT EXISTS idx_token_stats_token_id 
ON token_stats(token_id);

-- Index for first_bought timestamps (used in duration calculations)
CREATE INDEX IF NOT EXISTS idx_token_balance_first_bought 
ON token_balance(first_bought) WHERE volume > 0; 