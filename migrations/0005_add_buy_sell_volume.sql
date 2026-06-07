-- Migration to add buy_volume and sell_volume columns
-- This separates volume tracking for better reputation scoring

-- Add buy_volume and sell_volume to token_balance table
ALTER TABLE token_balance 
ADD COLUMN buy_volume NUMERIC(75,4) NOT NULL DEFAULT 0,
ADD COLUMN sell_volume NUMERIC(75,4) NOT NULL DEFAULT 0;

-- Add buy_volume and sell_volume to token_stats table
ALTER TABLE token_stats 
ADD COLUMN mean_buy_volume NUMERIC(75,4) DEFAULT 0,
ADD COLUMN mean_sell_volume NUMERIC(75,4) DEFAULT 0,
ADD COLUMN stddev_buy_volume NUMERIC(75,4) DEFAULT 1,
ADD COLUMN stddev_sell_volume NUMERIC(75,4) DEFAULT 1;

-- Add z-score columns for buy and sell volume
ALTER TABLE token_balance 
ADD COLUMN buy_volume_z NUMERIC(75,4) DEFAULT 0,
ADD COLUMN sell_volume_z NUMERIC(75,4) DEFAULT 0;

-- Create indexes for better performance on the new columns
CREATE INDEX idx_token_balance_buy_volume ON token_balance(buy_volume);
CREATE INDEX idx_token_balance_sell_volume ON token_balance(sell_volume);
CREATE INDEX idx_token_stats_buy_volume ON token_stats(mean_buy_volume);
CREATE INDEX idx_token_stats_sell_volume ON token_stats(mean_sell_volume); 