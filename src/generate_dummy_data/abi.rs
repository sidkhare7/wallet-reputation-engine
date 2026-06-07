use alloy::{
    primitives::{Address, Bytes, U160, U256},
    sol,
    sol_types::{SolCall, SolValue},
};

use anyhow::Result;

sol! {
    enum MarketType {
        BONDING_CURVE,
        UNISWAP_POOL
    }

    struct MarketState {
        MarketType marketType;
        address marketAddress;
    }

    struct SecondaryRewards {
        uint256 totalAmountEth;
        uint256 totalAmountToken;
        uint256 creatorAmountEth;
        uint256 creatorAmountToken;
        uint256 protocolAmountEth;
        uint256 protocolAmountToken;
    }

    function buy(
        address recipient,
        address refundRecipient,
        address orderReferrer,
        string memory comment,
        MarketType expectedMarketType,
        uint256 minOrderSize,
        uint160 sqrtPriceLimitX96
    ) external payable returns (uint256);

    function sell(
        uint256 tokensToSell,
        address recipient,
        address orderReferrer,
        string memory comment,
        MarketType expectedMarketType,
        uint256 minPayoutSize,
        uint160 sqrtPriceLimitX96
    ) external returns (uint256);

    function burn(uint256 tokensToBurn) external;

    function getEthBuyQuote(uint256 amount) external view returns (uint256);

    function getTokenSellQuote(uint256 amount) external view returns (uint256);

    function state() external view returns (MarketState memory);

    function tokenURI() external view returns (string memory);

    function tokenCreator() external view returns (address);
}

pub fn decode_buy_response(response: Bytes) -> Result<U256> {
    let tokens_bought = U256::abi_decode(&response, false)?;
    Ok(tokens_bought)
}

pub fn decode_sell_response(response: Bytes) -> Result<U256> {
    let eth_received = U256::abi_decode(&response, false)?;
    Ok(eth_received)
}

pub fn decode_get_eth_buy_quote_response(response: Bytes) -> Result<U256> {
    let tokens_amount = U256::abi_decode(&response, false)?;
    Ok(tokens_amount)
}

pub fn decode_get_token_sell_quote_response(response: Bytes) -> Result<U256> {
    let eth_amount = U256::abi_decode(&response, false)?;
    Ok(eth_amount)
}

pub fn decode_state_response(response: Bytes) -> Result<(MarketType, Address)> {
    let market_state = MarketState::abi_decode(&response, false)?;
    Ok((market_state.marketType, market_state.marketAddress))
}

pub fn decode_token_uri_response(response: Bytes) -> Result<String> {
    let uri = String::abi_decode(&response, false)?;
    Ok(uri)
}

pub fn decode_token_creator_response(response: Bytes) -> Result<Address> {
    let creator = Address::abi_decode(&response, false)?;
    Ok(creator)
}

pub fn buy_calldata(
    recipient: Address,
    refund_recipient: Address,
    order_referrer: Address,
    comment: String,
    expected_market_type: MarketType,
    min_order_size: U256,
    sqrt_price_limit_x96: U160,
) -> Bytes {
    Bytes::from(
        buyCall {
            recipient,
            refundRecipient: refund_recipient,
            orderReferrer: order_referrer,
            comment,
            expectedMarketType: expected_market_type,
            minOrderSize: min_order_size,
            sqrtPriceLimitX96: sqrt_price_limit_x96,
        }
        .abi_encode(),
    )
}

pub fn sell_calldata(
    tokens_to_sell: U256,
    recipient: Address,
    order_referrer: Address,
    comment: String,
    expected_market_type: MarketType,
    min_payout_size: U256,
    sqrt_price_limit_x96: U160,
) -> Bytes {
    Bytes::from(
        sellCall {
            tokensToSell: tokens_to_sell,
            recipient,
            orderReferrer: order_referrer,
            comment,
            expectedMarketType: expected_market_type,
            minPayoutSize: min_payout_size,
            sqrtPriceLimitX96: sqrt_price_limit_x96,
        }
        .abi_encode(),
    )
}

pub fn burn_calldata(tokens_to_burn: U256) -> Bytes {
    Bytes::from(burnCall { tokensToBurn: tokens_to_burn }.abi_encode())
}

pub fn get_eth_buy_quote_calldata(amount: U256) -> Bytes {
    Bytes::from(getEthBuyQuoteCall { amount }.abi_encode())
}

pub fn get_token_sell_quote_calldata(amount: U256) -> Bytes {
    Bytes::from(getTokenSellQuoteCall { amount }.abi_encode())
}

pub fn state_calldata() -> Bytes {
    Bytes::from(stateCall {}.abi_encode())
}

pub fn token_uri_calldata() -> Bytes {
    Bytes::from(tokenURICall {}.abi_encode())
}

pub fn token_creator_calldata() -> Bytes {
    Bytes::from(tokenCreatorCall {}.abi_encode())
}