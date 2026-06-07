-- Add reputation field to account table
ALTER TABLE account ADD COLUMN reputation INT DEFAULT 0;

-- Add index for efficient ordering by reputation
CREATE INDEX idx_account_reputation ON account(reputation);

-- Add comment for documentation
COMMENT ON COLUMN account.reputation IS 'Reputation score ranging from 0-100,000';
COMMENT ON COLUMN account.diamond_hand_probability IS 'Diamond hand probability score ranging from 0-500'; 