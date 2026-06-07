// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

interface ICult {
    /// @notice Thrown when an operation is attempted with a zero address
    error AddressZero();

    /// @notice Thrown when an invalid market type is specified
    error InvalidMarketType();

    /// @notice Thrown when there are insufficient funds for an operation
    error InsufficientFunds();

    /// @notice Thrown when there is insufficient liquidity for a transaction
    error InsufficientLiquidity();

    /// @notice Thrown when the slippage bounds are exceeded during a transaction
    error SlippageBoundsExceeded();

    /// @notice Thrown when the initial order size is too large
    error InitialOrderSizeTooLarge();

    /// @notice Thrown when the ETH amount is too small for a transaction
    error EthAmountTooSmall();

    /// @notice Thrown when an ETH transfer fails
    error EthTransferFailed();

    /// @notice Thrown when an operation is attempted by an entity other than the pool
    error OnlyPool();

    /// @notice Thrown when an operation is attempted by an entity other than WETH
    error OnlyWeth();

    /// @notice Thrown when a market is not yet graduated
    error MarketNotGraduated();

    /// @notice Thrown when a market is already graduated
    error MarketAlreadyGraduated();

    /// @notice Thrown when an operation is attempted by an entity other than the token creator
    error OnlyTokenCreator();

    /// @notice Represents the type of market
    enum MarketType {
        BONDING_CURVE,
        UNISWAP_POOL
    }

    /// @notice Represents the state of the market
    struct MarketState {
        MarketType marketType;
        address marketAddress;
    }

    /// @notice The secondary rewards accrued from the market's liquidity position
    struct SecondaryRewards {
        uint256 totalAmountEth;
        uint256 totalAmountToken;
        uint256 creatorAmountEth;
        uint256 creatorAmountToken;
        uint256 protocolAmountEth;
        uint256 protocolAmountToken;
    }

    /// @notice Emitted when secondary rewards are distributed
    event CultTokenSecondaryRewards(SecondaryRewards rewards);

    /// @notice Emitted when Cult tokens are transferred
    /// @param from The address of the sender
    /// @param to The address of the recipient
    /// @param amount The amount of tokens transferred
    /// @param fromTokenBalance The token balance of the sender after the transfer
    /// @param toTokenBalance The token balance of the recipient after the transfer
    /// @param totalSupply The total supply of tokens after the transfer
    event CultTokenTransfer(
        address indexed from,
        address indexed to,
        uint256 amount,
        uint256 fromTokenBalance,
        uint256 toTokenBalance,
        uint256 totalSupply
    );

    // function transfer(address to, uint256 amount) external returns (bool);
    /// @notice Allows a holder to burn their tokens after the market has graduated
    /// @dev Emits a CultTokenTransfer event with the updated token balances and total supply
    /// @param tokensToBurn The number of tokens to burn
    function burn(uint256 tokensToBurn) external;

    /// @notice Returns the URI of the token
    /// @return The token URI
    function tokenURI() external view returns (string memory);

    /// @notice Returns the address of the token creator
    /// @return The token creator's address
    function tokenCreator() external view returns (address);

    /// @notice Buys tokens from the bonding curve or Uniswap V3 pool depending on the market state.
    /// @param minOrderSize The minimum size of the order to prevent slippage, ignored if market is uniswap pool.
    /// @param sqrtPriceLimitX96 The price limit for Uniswap V3 pool swaps, ignored if market is bonding curve.
    function buy(
        uint256 minOrderSize,
        uint160 sqrtPriceLimitX96
    ) external payable;

    /// @notice Sells tokens to the bonding curve or Uniswap V3 pool depending on the market state
    /// @param tokensToSell The number of tokens to sell
    /// @param minPayoutSize The minimum payout size to prevent slippage, ignored if market is uniswap pool.
    /// @param sqrtPriceLimitX96 The price limit for Uniswap V3 pool swaps, ignored if market is bonding curve.
    function sell(
        uint256 tokensToSell,
        uint256 minPayoutSize,
        uint160 sqrtPriceLimitX96
    ) external;

    /// @notice Emitted when a market graduates
    /// @param tokenAddress The address of the token
    /// @param poolAddress The address of the pool
    /// @param totalEthLiquidity The total ETH liquidity in the pool
    /// @param totalTokenLiquidity The total token liquidity in the pool
    /// @param lpPositionId The ID of the liquidity position
    /// @param marketType The type of market
    event CultMarketGraduated(
        address indexed tokenAddress,
        address indexed poolAddress,
        uint256 totalEthLiquidity,
        uint256 totalTokenLiquidity,
        uint256 lpPositionId,
        MarketType marketType
    );
}
