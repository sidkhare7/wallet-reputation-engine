use crate::handlers::{
    CultSwapEvent, CultTokenCreatedEvent, CultTokenTransferEvent
};
use crate::utils::misc::to_checksum_address;
use anyhow::Result;
use bigdecimal::BigDecimal;
use std::str::FromStr;
use bigdecimal::ToPrimitive;

// Direct deserialization struct for incoming create token webhook data
#[derive(Debug, serde::Deserialize)]
pub struct IncomingCreateTokenData {
    pub token_address: String,
    pub token_creator: String,
    pub airdrop_contract: String,
    pub factory_address: String,
    pub protocol_fee_recipient: String,
    pub token_uri: String,
    pub name: String,
    pub symbol: String,
    pub pool_address: String,
    pub block_number: serde_json::Value, // Can be number (int/float) or string
    pub timestamp: serde_json::Value,    // Can be number (int/float) or string
    pub transaction_hash: String,
    pub merkle_roots: serde_json::Value,  // Can be string or array of strings
    pub total_amount: serde_json::Value,  // Can be number or string
    pub total_airdrop_recipient_count: serde_json::Value,  // Can be number (int/float) or string
}

// Direct deserialization struct for incoming token transfer webhook data
#[derive(Debug, serde::Deserialize)]
pub struct IncomingTransferData {
    pub from: String,
    pub to: String,
    pub amount: serde_json::Value, // Can be number or string
    pub from_token_balance: serde_json::Value, // Can be number or string
    pub to_token_balance: serde_json::Value, // Can be number or string
    pub total_supply: serde_json::Value, // Can be number or string
    pub timestamp: serde_json::Value, // Can be number (int/float) or string
    pub block_number: serde_json::Value, // Can be number (int/float) or string
    pub transaction_hash: String,
    #[serde(rename = "contract_id")]
    pub token_id: String,
}

// Direct deserialization struct for incoming swap webhook data
#[derive(Debug, serde::Deserialize)]
pub struct IncomingSwapData {
    pub sender: String,
    pub recipient: String,
    pub amount_0: serde_json::Value,      // Can be number or string
    pub amount_1: serde_json::Value,      // Can be number or string
    pub sqrt_price_x96: serde_json::Value, // Can be number or string
    pub liquidity: serde_json::Value,    // Can be number or string
    pub tick: serde_json::Value,         // Can be number or string
    pub timestamp: serde_json::Value,    // Can be number (int/float) or string
    pub block_number: serde_json::Value, // Can be number (int/float) or string
    pub transaction_hash: String,
    #[serde(rename = "contract_id")]
    pub pool_address: String,
}

impl IncomingCreateTokenData {
    pub fn to_cult_token_created_event(self) -> Result<CultTokenCreatedEvent, anyhow::Error> {
        // Helper function to convert serde_json::Value to u128 for total_amount
        let parse_u128 = |value: serde_json::Value| -> Result<u128, anyhow::Error> {
            match value {
                serde_json::Value::Number(n) => {
                    if let Some(u) = n.as_u64() {
                        Ok(u as u128)
                    } else if let Some(f) = n.as_f64() {
                        if f.is_finite() && f >= 0.0 {
                            Ok(f as u128)
                        } else {
                            Err(anyhow::anyhow!("Invalid float value: {}", f))
                        }
                    } else {
                        Err(anyhow::anyhow!("Number too large or invalid"))
                    }
                },
                serde_json::Value::String(s) => {
                    // Try parsing as u128 directly
                    s.parse::<u128>()
                        .or_else(|_| {
                            // Handle scientific notation strings
                            s.parse::<f64>()
                                .map(|f| f as u128)
                                .map_err(|e| anyhow::anyhow!("Failed to parse string '{}' as number: {}", s, e))
                        })
                        .map_err(|e| anyhow::anyhow!("Failed to parse string '{}': {}", s, e))
                },
                _ => Err(anyhow::anyhow!("Expected number or string, got: {:?}", value))
            }
        };

        // Helper function to convert serde_json::Value to u64 for timestamps and block numbers
        let parse_u64 = |value: serde_json::Value| -> Result<u64, anyhow::Error> {
            match value {
                serde_json::Value::Number(n) => {
                    if let Some(u) = n.as_u64() {
                        Ok(u)
                    } else if let Some(f) = n.as_f64() {
                        if f.is_finite() && f >= 0.0 {
                            Ok(f as u64)
                        } else {
                            Err(anyhow::anyhow!("Invalid float value: {}", f))
                        }
                    } else if let Some(i) = n.as_i64() {
                        if i >= 0 {
                            Ok(i as u64)
                        } else {
                            Err(anyhow::anyhow!("Negative value not allowed: {}", i))
                        }
                    } else {
                        Err(anyhow::anyhow!("Number too large or invalid"))
                    }
                },
                serde_json::Value::String(s) => {
                    s.parse::<u64>()
                        .or_else(|_| {
                            // Handle decimal strings by parsing as float first
                            s.parse::<f64>()
                                .map(|f| f as u64)
                                .map_err(|e| anyhow::anyhow!("Failed to parse string '{}' as number: {}", s, e))
                        })
                        .map_err(|e| anyhow::anyhow!("Failed to parse string '{}': {}", s, e))
                },
                _ => Err(anyhow::anyhow!("Expected number or string, got: {:?}", value))
            }
        };

        // Helper function to convert serde_json::Value to Vec<String> for merkle_roots
        let parse_merkle_roots = |value: serde_json::Value| -> Result<Vec<String>, anyhow::Error> {
            match value {
                serde_json::Value::String(s) => {
                    // Single string case - wrap in a vector
                    Ok(vec![s])
                },
                serde_json::Value::Array(arr) => {
                    // Array case - convert each element to string
                    let mut result = Vec::new();
                    for item in arr {
                        match item {
                            serde_json::Value::String(s) => result.push(s),
                            _ => return Err(anyhow::anyhow!("All array elements must be strings, got: {:?}", item))
                        }
                    }
                    Ok(result)
                },
                _ => Err(anyhow::anyhow!("Expected string or array of strings, got: {:?}", value))
            }
        };

        Ok(CultTokenCreatedEvent {
            token_address: to_checksum_address(&self.token_address)?,
            token_creator: to_checksum_address(&self.token_creator)?,
            airdrop_contract: to_checksum_address(&self.airdrop_contract)?,
            factory_address: to_checksum_address(&self.factory_address)?,
            protocol_fee_recipient: to_checksum_address(&self.protocol_fee_recipient)?,
            token_uri: self.token_uri,
            name: self.name,
            symbol: self.symbol,
            pool_address: to_checksum_address(&self.pool_address)?,
            block_number: parse_u64(self.block_number)?,
            block_timestamp: parse_u64(self.timestamp)?,
            transaction_hash: self.transaction_hash,
            chain_id: "10143".to_string(), // Hardcoded as per original transform function
            merkle_roots: parse_merkle_roots(self.merkle_roots)?, // Parse string or array of strings
            total_amount: parse_u128(self.total_amount)?,
            total_airdrop_recipient_count: parse_u64(self.total_airdrop_recipient_count)? as u32,
        })
    }
}


impl IncomingTransferData {
    pub fn to_cult_token_transfer_event(self) -> Result<CultTokenTransferEvent, anyhow::Error> {
        // Helper function to convert serde_json::Value to u128
        let parse_u128 = |value: serde_json::Value| -> Result<u128, anyhow::Error> {
            match value {
                serde_json::Value::Number(n) => {
                    if let Some(u) = n.as_u64() {
                        Ok(u as u128)
                    } else if let Some(f) = n.as_f64() {
                        if f.is_finite() && f >= 0.0 {
                            Ok(f as u128)
                        } else {
                            Err(anyhow::anyhow!("Invalid float value: {}", f))
                        }
                    } else {
                        Err(anyhow::anyhow!("Number too large or invalid"))
                    }
                }
                serde_json::Value::String(s) => {
                    // Try parsing as u128 directly
                    s.parse::<u128>()
                        .or_else(|_| {
                            // Handle scientific notation strings
                            s.parse::<f64>().map(|f| f as u128).map_err(|e| {
                                anyhow::anyhow!("Failed to parse string '{}' as number: {}", s, e)
                            })
                        })
                        .map_err(|e| anyhow::anyhow!("Failed to parse string '{}': {}", s, e))
                }
                _ => Err(anyhow::anyhow!(
                    "Expected number or string, got: {:?}",
                    value
                )),
            }
        };

        // Helper function to convert serde_json::Value to u64 for timestamps and block numbers
        let parse_u64 = |value: serde_json::Value| -> Result<u64, anyhow::Error> {
            match value {
                serde_json::Value::Number(n) => {
                    if let Some(u) = n.as_u64() {
                        Ok(u)
                    } else if let Some(f) = n.as_f64() {
                        if f.is_finite() && f >= 0.0 {
                            Ok(f as u64)
                        } else {
                            Err(anyhow::anyhow!("Invalid float value: {}", f))
                        }
                    } else {
                        Err(anyhow::anyhow!("Number too large or invalid"))
                    }
                }
                serde_json::Value::String(s) => {
                    // Try parsing as u64 directly
                    s.parse::<u64>()
                        .or_else(|_| {
                            // Handle scientific notation strings
                            s.parse::<f64>().map(|f| f as u64).map_err(|e| {
                                anyhow::anyhow!("Failed to parse string '{}' as number: {}", s, e)
                            })
                        })
                        .map_err(|e| anyhow::anyhow!("Failed to parse string '{}': {}", s, e))
                },
                _ => Err(anyhow::anyhow!(
                    "Expected number or string, got: {:?}",
                    value
                )),
            }
        };

        Ok(CultTokenTransferEvent {
            from: to_checksum_address(&self.from)?,
            to: to_checksum_address(&self.to)?,
            from_token_balance: parse_u128(self.from_token_balance)?,
            to_token_balance: parse_u128(self.to_token_balance)?,
            block_timestamp: parse_u64(self.timestamp)?,
            transaction_hash: self.transaction_hash,
            token_id: to_checksum_address(&self.token_id)?,
        })
    }
}

impl IncomingSwapData {
    pub fn to_cult_swap_event(self) -> Result<CultSwapEvent, anyhow::Error> {
        // Unified number parser that preserves precision
        let parse_number = |value: serde_json::Value| -> Result<String, anyhow::Error> {
            match value {
                // Handle scientific notation in numbers by converting to string first
                serde_json::Value::Number(n) => Ok(n.to_string()),
                serde_json::Value::String(s) => Ok(s),
                _ => Err(anyhow::anyhow!("Expected number or string, got: {:?}", value)),
            }
        };

        // Enhanced BigDecimal parser
        let parse_bigdecimal = |value: serde_json::Value| -> Result<BigDecimal, anyhow::Error> {
            parse_number(value)?
                .parse::<BigDecimal>()
                .map_err(|e| anyhow::anyhow!("BigDecimal parse error: {}", e))
        };

        // Enhanced i64 parser with range checking
        let parse_i64 = |value: serde_json::Value| -> Result<i64, anyhow::Error> {
            let s = parse_number(value)?;
            s.parse::<i64>()
                .or_else(|_| {
                    BigDecimal::from_str(&s)?
                        .to_i64()
                        .ok_or_else(|| anyhow::anyhow!("Value out of i64 range: {}", s))
                })
                .map_err(|e| anyhow::anyhow!("i64 parse error: {}", e))
        };

        // Enhanced u64 parser with range checking
        let parse_u64 = |value: serde_json::Value| -> Result<u64, anyhow::Error> {
            let s = parse_number(value)?;
            s.parse::<u64>()
                .or_else(|_| {
                    BigDecimal::from_str(&s)?
                        .to_u64()
                        .ok_or_else(|| anyhow::anyhow!("Value out of u64 range: {}", s))
                })
                .map_err(|e| anyhow::anyhow!("u64 parse error: {}", e))
        };

        Ok(CultSwapEvent {
            sender: to_checksum_address(&self.sender)?,
            recipient: to_checksum_address(&self.recipient)?,
            amount0: parse_bigdecimal(self.amount_0)?,
            amount1: parse_bigdecimal(self.amount_1)?,
            sqrt_price_x96: parse_bigdecimal(self.sqrt_price_x96)?,
            liquidity: parse_bigdecimal(self.liquidity)?,
            tick: parse_i64(self.tick)?,
            pool_address: to_checksum_address(&self.pool_address)?,
            transaction_hash: self.transaction_hash,
            block_timestamp: parse_u64(self.timestamp)?,
        })
    }
}

// Batch wrapper types for handling both single and array webhook data
#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum BatchCreateTokenData {
    Single(IncomingCreateTokenData),
    Batch(Vec<IncomingCreateTokenData>),
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum BatchTransferData {
    Single(IncomingTransferData),
    Batch(Vec<IncomingTransferData>),
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum BatchSwapData {
    Single(IncomingSwapData),
    Batch(Vec<IncomingSwapData>),
}

// Batch response types
#[derive(Debug, serde::Serialize)]
pub struct BatchWebhookResponse {
    pub status: String,
    pub processed: usize,
    pub failed: usize,
}

impl BatchCreateTokenData {
    pub fn to_vec(self) -> Vec<IncomingCreateTokenData> {
        match self {
            BatchCreateTokenData::Single(data) => vec![data],
            BatchCreateTokenData::Batch(data) => data,
        }
    }
}

impl BatchTransferData {
    pub fn to_vec(self) -> Vec<IncomingTransferData> {
        match self {
            BatchTransferData::Single(data) => vec![data],
            BatchTransferData::Batch(data) => data,
        }
    }
}

impl BatchSwapData {
    pub fn to_vec(self) -> Vec<IncomingSwapData> {
        match self {
            BatchSwapData::Single(data) => vec![data],
            BatchSwapData::Batch(data) => data,
        }
    }
}
