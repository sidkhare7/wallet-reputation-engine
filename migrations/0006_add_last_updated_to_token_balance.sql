-- Migration to add last_updated column to token_balance table
-- This tracks when the record was last modified for holding duration logic

-- Add last_updated column to token_balance table
ALTER TABLE token_balance 
ADD COLUMN last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Update existing records to have last_updated set to first_bought
UPDATE token_balance 
SET last_updated = first_bought 
WHERE last_updated IS NULL; 