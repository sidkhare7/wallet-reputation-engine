// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import {Initializable} from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import {IERC721Receiver} from "@openzeppelin/contracts/token/ERC721/IERC721Receiver.sol";
import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
import {ReentrancyGuardUpgradeable} from "@openzeppelin/contracts-upgradeable/utils/ReentrancyGuardUpgradeable.sol";
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {ICult} from "./interfaces/ICult.sol";
import {INonfungiblePositionManager} from "./interfaces/INonfungiblePositionManager.sol";
import {IUniswapV3Pool} from "./interfaces/IUniswapV3Pool.sol";
import {ISwapRouter} from "./interfaces/ISwapRouter.sol";
import {ICultRewards} from "./interfaces/ICultRewards.sol";
import {IWETH} from "./interfaces/IWETH.sol";

/**
 * @title Cult
 * @dev Implementation of the Cult token with bonding curve and Uniswap V3 pool functionalities.
 */
contract Cult is ICult, Initializable, ERC20Upgradeable, ReentrancyGuardUpgradeable, OwnableUpgradeable, IERC721Receiver {
    /// ==================== Constants ==================== ///
    /// @notice Maximum total supply of the Cult token (10B tokens)
    uint256 public constant MAX_TOTAL_SUPPLY = 10_000_000_000e18;
    
    
    /// @notice Supply airdropped when token got deployed
    uint256 public AIRDROPPED_SUPPLY;
    /// @notice Supply allocated for the secondary market (Max - (total_supply))
    uint256 public SECONDARY_MARKET_SUPPLY;
    /// @notice Total fee in basis points (bps) 1%
    // uint256 public constant TOTAL_FEE_BPS = 10000;
    /// @notice Fee percentage for the token creator, as a portion of TOTAL_FEE_BPS (25% = 250000)
    /// @notice Initial square root price for the WETH token in the Uniswap V3 pool.
    // uint160 internal constant POOL_SQRT_PRICE_X96_WETH_0 = 400950665883918763141200546267337;
    // uint160 internal constant POOL_SQRT_PRICE_X96_WETH_0 = 257203292402732883117133571268608;
    uint160 public POOL_SQRT_PRICE_X96_WETH_0;
    /// @notice Initial square root price for the Cult token in the Uniswap V3 pool.
    // uint160 internal constant POOL_SQRT_PRICE_X96_TOKEN_0 = 15655546353934715619853339;
    // uint160 internal constant POOL_SQRT_PRICE_X96_TOKEN_0 = 198141505324145;
    uint160 public POOL_SQRT_PRICE_X96_TOKEN_0;
    /// @notice Liquidity provider fee in basis points for the Uniswap V3 pool.
    uint24 internal constant LP_FEE = 500; // 0.05%
    /// @notice Lower tick boundary for the Uniswap V3 pool.
    int24 internal constant LP_TICK_LOWER = -887200;
    /// @notice Upper tick boundary for the Uniswap V3 pool.
    int24 internal constant LP_TICK_UPPER = 887200;

    /// ==================== State variables ==================== ///
    /// @notice Address of the WETH token.
    address public immutable WETH;
    /// @notice Address of the Uniswap V3 non-fungible position manager.
    address public immutable nonfungiblePositionManager;
    /// @notice Address of the Uniswap V3 swap router.
    address public immutable swapRouter;
    /// @notice Address to receive protocol fees.
    address public immutable protocolFeeRecipient;
    /// @notice Address of the Cult rewards contract.
    address public immutable cultRewards;

    /// @notice Address of the Uniswap V3 pool.
    address public poolAddress;
    /// @notice Address of the token creator.
    address public tokenCreator;
    /// @notice Token ID for the liquidity position in Uniswap V3.
    uint256 public lpTokenId;
    /// @notice URI for the ERC20 token metadata.
    string public tokenURI;
    // /// @notice Merkle root for airdrop verification.
    // bytes32[] public merkleRoots;
    /// @notice Current market type (BONDING_CURVE or UNISWAP_POOL).
    MarketType public marketType;

    /// ==================== Constructor ==================== ///
    /// @notice Constructor to set immutable addresses.
    /// @param _protocolFeeRecipient Address to receive protocol fees.
    /// @param _cultRewards Address of the Cult rewards contract.
    /// @param _weth Address of the WETH token.
    /// @param _nonfungiblePositionManager Address of the Uniswap V3 position manager.
    /// @param _swapRouter Address of the Uniswap V3 swap router.
    constructor(
        address _protocolFeeRecipient,
        address _cultRewards,
        address _weth,
        address _nonfungiblePositionManager,
        address _swapRouter
    ) {
        if (_protocolFeeRecipient == address(0)) revert AddressZero();
        if (_cultRewards == address(0)) revert AddressZero();
        if (_weth == address(0)) revert AddressZero();
        if (_nonfungiblePositionManager == address(0)) revert AddressZero();
        if (_swapRouter == address(0)) revert AddressZero();

        protocolFeeRecipient = _protocolFeeRecipient;
        cultRewards = _cultRewards;
        WETH = _weth;
        nonfungiblePositionManager = _nonfungiblePositionManager;
        swapRouter = _swapRouter;
    }

    /// ==================== Public Functions ==================== ///

    /// ==================== Initializer ==================== ///
    /// @notice Initializes a new Cult token.
    /// @param _tokenCreator The address of the token creator.
    /// @param _owner The address of the contract owner.
    /// @param _tokenURI The ERC20 token URI.
    /// @param _name The token name.
    /// @param _symbol The token symbol.
    // @param _merkleRoots The merkle roots for airdrop verification.
    // /// @param airdropAmount The amount of tokens to airdrop.
    // /// @param airdropContract The address of the airdrop contract.
    function initialize(
        address _tokenCreator,
        address _owner,
        // address airdropContract,
        string memory _tokenURI,
        string memory _name,
        string memory _symbol,
        // bytes32[] calldata _merkleRoots,
        // uint256 airdropAmount,
        uint160 sqrtPriceX96_WETH_0,
        uint160 sqrtPriceX96_TOKEN_0
    ) public payable initializer {
        // Set the merkle roots
        // merkleRoots = _merkleRoots;

        // Validate the creation parameters
        if (_tokenCreator == address(0)) revert AddressZero();
        if (_owner == address(0)) revert AddressZero();

        // Initialize base contract state
        __ERC20_init(_name, _symbol);
        __ReentrancyGuard_init();
        __Ownable_init(_owner);

        // Initialize token and market state
        marketType = MarketType.UNISWAP_POOL;
        tokenCreator = _tokenCreator;
        tokenURI = _tokenURI;

        // Determine the token0, token1, and sqrtPriceX96 values for the Uniswap V3 pool
        // address token0 = WETH < address(this) ? WETH : address(this);
        // address token1 = WETH < address(this) ? address(this) : WETH;
        POOL_SQRT_PRICE_X96_WETH_0 = sqrtPriceX96_WETH_0;
        POOL_SQRT_PRICE_X96_TOKEN_0 = sqrtPriceX96_TOKEN_0;
        // uint160 sqrtPriceX96 = token0 == WETH ? sqrtPriceX96_WETH_0 : sqrtPriceX96_TOKEN_0;

        //Minting to airdrop contract, similar to buy operation
        //_mint(airdropContract, airdropAmount);
        // _mint(airdropContract, airdropAmount);
        // AIRDROPPED_SUPPLY = airdropAmount;
        // _mint(address(this), MAX_TOTAL_SUPPLY - airdropAmount);
        _mint(tokenCreator, MAX_TOTAL_SUPPLY);
        // Create and initialize the Uniswap V3 pool
        // poolAddress = INonfungiblePositionManager(nonfungiblePositionManager).createAndInitializePoolIfNecessary(
        //     token0, token1, LP_FEE, sqrtPriceX96
        // );
    }

    /// @notice Purchases tokens using ETH, either from the bonding curve or Uniswap V3 pool.
    /// @param minOrderSize The minimum tokens to prevent slippage.
    /// @param sqrtPriceLimitX96 The price limit for Uniswap V3 pool swaps, ignored if market is bonding curve.
    function buy(
        uint256 minOrderSize,
        uint160 sqrtPriceLimitX96
    ) public payable nonReentrant {
        // Calculate the remaining ETH
        uint256 totalCost = msg.value;

        // Convert the ETH to WETH and approve the swap router
        IWETH(WETH).deposit{value: totalCost}();
        IWETH(WETH).approve(swapRouter, totalCost);

        // Set up the swap parameters
        ISwapRouter.ExactInputSingleParams memory params = ISwapRouter.ExactInputSingleParams({
            tokenIn: WETH,
            tokenOut: address(this),
            fee: LP_FEE,
            recipient: msg.sender,
            amountIn: totalCost,
            amountOutMinimum: minOrderSize,
            sqrtPriceLimitX96: sqrtPriceLimitX96
        });

        ISwapRouter(swapRouter).exactInputSingle(params);
        
    }

    /// @notice Sells tokens for ETH on Uniswap V3 pool.
    /// @param tokensToSell The number of tokens to sell.
    /// @param minPayoutSize The minimum ETH payout to prevent slippage.
    /// @param sqrtPriceLimitX96 The price limit for Uniswap V3 pool swaps, ignored if market is bonding curve.
    function sell(
        uint256 tokensToSell,
        uint256 minPayoutSize,
        uint160 sqrtPriceLimitX96
    ) external nonReentrant {
        // Ensure the sender has enough liquidity to sell
        if (tokensToSell > balanceOf(msg.sender)) {
            revert InsufficientLiquidity();
        }

        transfer(address(this), tokensToSell);
        this.approve(swapRouter, tokensToSell);

        // Set up the swap parameters
        ISwapRouter.ExactInputSingleParams memory params = ISwapRouter.ExactInputSingleParams({
            tokenIn: address(this),
            tokenOut: WETH,
            fee: LP_FEE,
            recipient: address(this),
            amountIn: tokensToSell,
            amountOutMinimum: minPayoutSize,
            sqrtPriceLimitX96: sqrtPriceLimitX96
        });

        // Execute the swap
        uint256 payout = ISwapRouter(swapRouter).exactInputSingle(params);

        // Withdraw the ETH from the contract
        IWETH(WETH).withdraw(payout);

        // Send the payout to the recipient
        (bool success,) = msg.sender.call{value: payout}("");
        if (!success) revert EthTransferFailed();
    }

    /// @notice Withdraws ETH held by this contract to a specified address. Pass amount == type(uint256).max to withdraw full balance.
    /// @param to Recipient address
    /// @param amount Amount of ETH to withdraw
    function withdrawETH(address to, uint256 amount) external nonReentrant onlyOwner {
        if (to == address(0)) revert AddressZero();

        uint256 balance = address(this).balance;
        uint256 amountToSend = amount == type(uint256).max ? balance : amount;
        if (amountToSend > balance) amountToSend = balance;

        (bool success, ) = payable(to).call{value: amountToSend}("");
        if (!success) revert EthTransferFailed();
    }

    /// @notice Withdraws WETH tokens held by this contract to a specified address. Pass amount == type(uint256).max to withdraw full balance.
    /// @param to Recipient address
    /// @param amount Amount of WETH to withdraw
    function withdrawWETH(address to, uint256 amount) external nonReentrant onlyOwner {
        if (to == address(0)) revert AddressZero();

        uint256 balance = IERC20(WETH).balanceOf(address(this));
        uint256 amountToSend = amount == type(uint256).max ? balance : amount;
        if (amountToSend > balance) amountToSend = balance;

        SafeERC20.safeTransfer(IERC20(WETH), to, amountToSend);
    }

    /// @notice Withdraws arbitrary ERC20 tokens held by this contract to a specified address. Pass amount == type(uint256).max to withdraw full balance.
    /// @param tokenAddress ERC20 token address
    /// @param to Recipient address
    /// @param amount Amount of tokens to withdraw
    function withdrawToken(address tokenAddress, address to, uint256 amount) external nonReentrant onlyOwner {
        if (to == address(0) || tokenAddress == address(0)) revert AddressZero();

        uint256 balance = IERC20(tokenAddress).balanceOf(address(this));
        uint256 amountToSend = amount == type(uint256).max ? balance : amount;
        if (amountToSend > balance) amountToSend = balance;

        SafeERC20.safeTransfer(IERC20(tokenAddress), to, amountToSend);
    }


    /// @notice Transfers tokens to a specified address.
    /// @param to The address to transfer to.
    /// @param value The amount to be transferred.
    /// @return A boolean indicating success.
    function transfer(address to, uint256 value) public override(ERC20Upgradeable) returns (bool) {
        return super.transfer(to, value); // Calls the ERC20 transfer
    }

    /// @notice Burns tokens after the market has graduated to Uniswap V3.
    /// @param tokensToBurn The number of tokens to burn.
    function burn(uint256 tokensToBurn) external {
        _burn(msg.sender, tokensToBurn);
    }

    /// @notice receives ETH so it can graduate manually
    receive() external payable {
        if (msg.sender == WETH) {
            return;
        }
        //revert("Not allowed");
        //buy(msg.sender, msg.sender, address(0), "", marketType, 0, 0);
    }

    /// @dev For receiving the Uniswap V3 LP NFT on market graduation.
    /// @return The selector to confirm the token transfer.
    function onERC721Received(address, address, uint256, bytes calldata) external view returns (bytes4) {
        if (msg.sender != poolAddress) revert OnlyPool();

        return this.onERC721Received.selector;
    }

    /// @dev No-op to allow a swap on the pool to set the correct initial price, if needed.
    /// @param amount0Delta The change in token0 balance of the pool.
    /// @param amount1Delta The change in token1 balance of the pool.
    /// @param data Additional data with no specified format.
    function uniswapV3SwapCallback(int256 amount0Delta, int256 amount1Delta, bytes calldata data) external {}

    /// @notice Graduates the market from bonding curve to Uniswap V3 pool.
    /// @dev This function can only be called by the token creator and only when the market is in BONDING_CURVE state.
    function graduateMarket() external {
        if (msg.sender != tokenCreator) revert OnlyTokenCreator();
        _graduateMarket();
    }

    /// ==================== Internal Functions ==================== ///

    // Define the ERC20 storage location using the same slot as ERC20Upgradeable
    // keccak256(abi.encode(uint256(keccak256("openzeppelin.storage.ERC20")) - 1)) & ~bytes32(uint256(0xff))
    bytes32 private constant ERC20_STORAGE_LOCATION = 0x52c63247e1f47db19d5ce0460030c497f067ca4cebf71ba98eeadabe20bace00;

    function _getERC20StorageInternal() private pure returns (ERC20Upgradeable.ERC20Storage storage $) {
        assembly {
            $.slot := ERC20_STORAGE_LOCATION
        }
    }

    // @dev Overrides ERC20's _update function to
    //      - Prevent transfers to the pool if the market has not graduated.
    //      - Emit the superset `CultTokenTransfer` event with each ERC20 transfer.
    // @param from The address from which tokens are transferred.
    // @param to The address to which tokens are transferred.
    // @param value The amount of tokens transferred.
    function _update(address from, address to, uint256 value) internal virtual override {
        if (marketType == MarketType.BONDING_CURVE && to == poolAddress) {
            revert MarketNotGraduated();
        }

        super._update(from, to, value);

        emit CultTokenTransfer(from, to, value, balanceOf(from), balanceOf(to), totalSupply());
    }

    /// @dev Graduates the market to a Uniswap V3 pool.
    function _graduateMarket() internal {
        // // Update the market type
        // @note - already set in initialize
        // marketType = MarketType.UNISWAP_POOL;

        // Convert the bonding curve's accumulated ETH to WETH
        uint256 ethLiquidity = address(this).balance;
        IWETH(WETH).deposit{value: ethLiquidity}();
        SECONDARY_MARKET_SUPPLY = MAX_TOTAL_SUPPLY - totalSupply();
        // Mint the secondary market supply to this contract
        _mint(address(this), SECONDARY_MARKET_SUPPLY);

        // Approve the nonfungible position manager to transfer the WETH and tokens
        SafeERC20.safeIncreaseAllowance(IERC20(WETH), address(nonfungiblePositionManager), ethLiquidity);
        SafeERC20.safeIncreaseAllowance(this, address(nonfungiblePositionManager), SECONDARY_MARKET_SUPPLY);

        // Determine the token order
        bool isWethToken0 = address(WETH) < address(this);
        address token0 = isWethToken0 ? WETH : address(this);
        address token1 = isWethToken0 ? address(this) : WETH;
        uint256 amount0 = isWethToken0 ? ethLiquidity : SECONDARY_MARKET_SUPPLY;
        uint256 amount1 = isWethToken0 ? SECONDARY_MARKET_SUPPLY : ethLiquidity;

        // Get the current and desired price of the pool
        uint160 currentSqrtPriceX96 = IUniswapV3Pool(poolAddress).slot0().sqrtPriceX96;
        uint160 desiredSqrtPriceX96 = isWethToken0 ? POOL_SQRT_PRICE_X96_WETH_0 : POOL_SQRT_PRICE_X96_TOKEN_0;

        // If the current price is not the desired price, set the desired price
        if (currentSqrtPriceX96 != desiredSqrtPriceX96) {
            bool swap0To1 = currentSqrtPriceX96 > desiredSqrtPriceX96;
            IUniswapV3Pool(poolAddress).swap(address(this), swap0To1, 100, desiredSqrtPriceX96, "");
        }

        // Set up the liquidity position mint parameters
        INonfungiblePositionManager.MintParams memory params = INonfungiblePositionManager.MintParams({
            token0: token0,
            token1: token1,
            fee: LP_FEE,
            tickLower: LP_TICK_LOWER,
            tickUpper: LP_TICK_UPPER,
            amount0Desired: amount0,
            amount1Desired: amount1,
            amount0Min: 0,
            amount1Min: 0,
            recipient: address(this),
            deadline: block.timestamp
        });

        // Mint the liquidity position to this contract and store the token id.
        (lpTokenId,,,) = INonfungiblePositionManager(nonfungiblePositionManager).mint(params);
        _handleExcessAfterGraduation();

        emit CultMarketGraduated(
            address(this), poolAddress, ethLiquidity, SECONDARY_MARKET_SUPPLY, lpTokenId, marketType
        );
    }
    /// @dev Burns excess CULT tokens and transfers any remaining ETH to the token creator after graduation.
    /// @dev This function is called after the market has graduated to Uniswap V3.
    function _handleExcessAfterGraduation() internal {
        // unwrap leftover WETH as we wrapped all ETH in graduation
        uint256 wethLiquidity = IWETH(WETH).balanceOf(address(this));
        if (wethLiquidity > 0) {
            IWETH(WETH).withdraw(wethLiquidity);

            // transfer the ETH to the token creator
            (bool success, ) = tokenCreator.call{value: wethLiquidity}("");
            if (!success) revert EthTransferFailed();
        }

        // burn excess CULT tokens held by this contract
        uint256 cultBalance = balanceOf(address(this));
        if (cultBalance > 0) {
            _burn(address(this), cultBalance);
        }
    }

}
