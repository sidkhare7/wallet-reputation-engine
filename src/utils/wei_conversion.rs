// // src/utils/wei_conversion.rs
// use bigdecimal::BigDecimal;
// use ethnum::U256;
// use std::str::FromStr;
// use thiserror::Error;

// #[derive(Error, Debug)]
// pub enum ConversionError {
//     #[error("Invalid numeric format: {0}")]
//     InvalidFormat(String),
//     #[error("Value out of range for U256")]
//     OutOfRange,
// }

// /// Convert database NUMERIC (BigDecimal) to U256
// pub fn from_db(value: &BigDecimal) -> Result<U256, ConversionError> {
//     // Ensure we're working with an integer value
//     if value.fractional_digit_count() > 0 {
//         return Err(ConversionError::InvalidFormat(
//             "Wei value contains decimal places".into(),
//         ));
//     }

//     U256::from_dec_str(&value.to_string())
//         .map_err(|_| ConversionError::InvalidFormat("Failed to parse U256".into()))
// }

// /// Convert U256 to database NUMERIC (BigDecimal)
// pub fn to_db(value: U256) -> Result<BigDecimal, ConversionError> {
//     BigDecimal::from_str(&value.to_string())
//         .map_err(|_| ConversionError::InvalidFormat("Failed to create BigDecimal".into()))
// }
