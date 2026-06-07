-- CultToken Table
CREATE TABLE cult_token (
    --EVENT PARAMS
    id TEXT PRIMARY KEY, -- Changed to TEXT for address
    name TEXT NOT NULL,
    symbol TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    transaction_hash TEXT NOT NULL,
    airdrop_contract TEXT NOT NULL,
    total_amount NUMERIC(75,4) NOT NULL, 
    total_airdrop_recipient_count BIGINT NOT NULL,
    
    -- OTHER PARAMS
    ipfs_content TEXT NOT NULL,
    lpPositionId BIGINT DEFAULT 0,
    volume NUMERIC(75,4) DEFAULT 0
);

-- Communities Table (unchanged)
CREATE TABLE communities (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    img_url TEXT NOT NULL,
    address TEXT NOT NULL UNIQUE,
    chain TEXT NOT NULL,
    merkle_root BYTEA NOT NULL, 
    last_updated_time TIMESTAMPTZ, 
    merkle_proofs JSONB NOT NULL,
    holder_count BIGINT NOT NULL,
    community_score NUMERIC(5,2)
);

-- Account Table
CREATE TABLE account (
    id TEXT PRIMARY KEY, -- Storing address as text
    slug TEXT,
    referral_code TEXT,
    diamond_hand_probability INT NOT NULL CHECK (diamond_hand_probability >= 0), -- Ensure non-negative
    referrer_id TEXT REFERENCES account(id) ON DELETE SET NULL, -- Foreign key reference
    total_referrals INT DEFAULT 0, -- Ensure non-negative
    fee_collected NUMERIC(75,4) DEFAULT 0, -- Store large U256 values safely
    twitter TEXT DEFAULT NULL,
    discord TEXT DEFAULT NULL,
    tokens_created INTEGER DEFAULT 0,
    tokens_migrated INTEGER DEFAULT 0,
    is_registered BOOLEAN DEFAULT FALSE NOT NULL
);

-- 2. Create the corrected token_airdrops table
CREATE TABLE token_airdrops (
    id SERIAL PRIMARY KEY,
    transaction_hash TEXT NOT NULL, 
    token_id TEXT NOT NULL REFERENCES cult_token(id),
    merkle_root BYTEA NOT NULL,      -- Match communities.merkle_root type
    community_id TEXT NOT NULL REFERENCES communities(id),
    community_name TEXT NOT NULL,    -- Snapshot at airdrop time
    merkle_proofs JSONB NOT NULL,   -- Snapshot at airdrop time
    total_amount NUMERIC(75,4) NOT NULL,
    total_recipient_count BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE airdrop_recipients (
    account_id TEXT REFERENCES account(id),
    token_airdrop_id INTEGER REFERENCES token_airdrops(id),
    claimed BOOLEAN DEFAULT FALSE NOT NULL,
    PRIMARY KEY (account_id, token_airdrop_id)
);

CREATE INDEX idx_airdrop_recipients_account ON airdrop_recipients(account_id);
CREATE INDEX idx_airdrop_recipients_airdrop ON airdrop_recipients(token_airdrop_id);


-- 3. Create optimal indexes
CREATE INDEX idx_token_airdrops_token ON token_airdrops(token_id);
CREATE INDEX idx_token_airdrops_community_id ON token_airdrops(community_id);


CREATE TABLE token_stats (
    token_id TEXT PRIMARY KEY REFERENCES cult_token(id),
    
    mean_duration NUMERIC(75,4),
    mean_volume NUMERIC(75,4),
    mean_value NUMERIC(75,4),
    mean_pnl NUMERIC(75,4),
    
    stddev_duration NUMERIC(75,4),
    stddev_volume NUMERIC(75,4),
    stddev_value NUMERIC(75,4),
    stddev_pnl NUMERIC(75,4),

    liquidity_score NUMERIC(75,4),
    holder_weight NUMERIC(75,4)
);


CREATE TABLE diamond_hand_list (
    account_id TEXT PRIMARY KEY REFERENCES account(id),
    selected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    merkle_root BYTEA,  --not needed
    last_updated_time TIMESTAMPTZ, 
    merkle_proofs JSONB  --not needed
);

-- TokenBalance Table
CREATE TABLE token_balance (
    account_id TEXT REFERENCES account(id),
    token_id TEXT REFERENCES cult_token(id),
    first_bought TIMESTAMPTZ NOT NULL,
    volume NUMERIC(75,4) NOT NULL,
    holding_duration BIGINT,
    pnl NUMERIC(75,4) NOT NULL,
    cost_basis NUMERIC(75,4) NOT NULL,
    holdings_value NUMERIC(75,4) NOT NULL,
    duration_z NUMERIC(75,4),
    pnl_z NUMERIC(75,4),
    value_z NUMERIC(75,4),
    volume_z NUMERIC(75,4),
    PRIMARY KEY (account_id, token_id)
);



-- Account_Communities Table 
CREATE TABLE account_communities (
    account_id TEXT REFERENCES account(id),
    community_id TEXT REFERENCES communities(id),
    PRIMARY KEY (account_id, community_id)
);

-- Materialized View for account_communities
CREATE MATERIALIZED VIEW account_community_summary AS
SELECT a.id, array_agg(c.address) AS communities
FROM account a
JOIN account_communities ac ON a.id = ac.account_id
JOIN communities c ON ac.community_id = c.id
GROUP BY a.id;

-- Materialized View for global_rankings
CREATE MATERIALIZED VIEW global_rankings AS
SELECT 
    a.id AS user_id,
    SUM(tb.duration_z * ts.liquidity_score) AS duration_score,
    SUM(tb.pnl_z * ts.holder_weight) AS pnl_score,
    SUM(tb.value_z * ts.liquidity_score) AS holdings_score,
    RANK() OVER (ORDER BY SUM(tb.duration_z * 0.4 + tb.pnl_z * 0.5 + tb.value_z * 0.1) DESC) AS global_rank
FROM token_balance tb
JOIN token_stats ts ON tb.token_id = ts.token_id
JOIN account a ON tb.account_id = a.id
GROUP BY a.id;