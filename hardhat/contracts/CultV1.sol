// // SPDX-License-Identifier: MIT
// pragma solidity ^0.8.23;

// import {Initializable} from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
// import {IERC721Receiver} from "@openzeppelin/contracts/token/ERC721/IERC721Receiver.sol";
// import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
// import {ReentrancyGuardUpgradeable} from "@openzeppelin/contracts-upgradeable/utils/ReentrancyGuardUpgradeable.sol";
// import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
// import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
// import {ICult} from "./interfaces/ICult.sol";
// import {INonfungiblePositionManager} from "./interfaces/INonfungiblePositionManager.sol";
// import {IUniswapV3Pool} from "./interfaces/IUniswapV3Pool.sol";
// import {ISwapRouter} from "./interfaces/ISwapRouter.sol";
// import {ICultRewards} from "./interfaces/ICultRewards.sol";
// import {IWETH} from "./interfaces/IWETH.sol";
// import {FullMath} from "./libraries/FullMath.sol";

// /**
//  * @title Cult
//  * @dev Implementation of the Cult token with bonding curve and Uniswap V3 pool functionalities.
//  */
// contract CultV1 is ICult, Initializable, ERC20Upgradeable, ReentrancyGuardUpgradeable, IERC721Receiver {
//     /// ==================== Constants ==================== ///
//     /// @notice Maximum total supply of the Cult token (10B tokens)
//     uint256 public constant MAX_TOTAL_SUPPLY = 10_000_000_000e18;
    
    
//     /// @notice Supply airdropped when token got deployed
//     uint256 public AIRDROPPED_SUPPLY;
//     /// @notice Supply allocated for the secondary market (Max - (total_supply))
//     uint256 public SECONDARY_MARKET_SUPPLY;
//     /// @notice Total fee in basis points (bps) 1%
//     // uint256 public constant TOTAL_FEE_BPS = 10000;
//     /// @notice Fee percentage for the token creator, as a portion of TOTAL_FEE_BPS (25% = 250000)
//     /// @notice Initial square root price for the WETH token in the Uniswap V3 pool.
//     // uint160 internal constant POOL_SQRT_PRICE_X96_WETH_0 = 400950665883918763141200546267337;
//     uint160 internal POOL_SQRT_PRICE_X96_WETH_0;
//     /// @notice Initial square root price for the Cult token in the Uniswap V3 pool.
//     // uint160 internal constant POOL_SQRT_PRICE_X96_TOKEN_0 = 15655546353934715619853339;
//     uint160 internal POOL_SQRT_PRICE_X96_TOKEN_0;
//     /// @notice Liquidity provider fee in basis points for the Uniswap V3 pool.
//     uint24 internal constant LP_FEE = 500; // 0.05%
//     /// @notice Lower tick boundary for the Uniswap V3 pool.
//     int24 internal constant LP_TICK_LOWER = -887200;
//     /// @notice Upper tick boundary for the Uniswap V3 pool.
//     int24 internal constant LP_TICK_UPPER = 887200;

//     /// ==================== State variables ==================== ///
//     /// @notice Address of the WETH token.
//     address public immutable WETH;
//     /// @notice Address of the Uniswap V3 non-fungible position manager.
//     address public immutable nonfungiblePositionManager;
//     /// @notice Address of the Uniswap V3 swap router.
//     address public immutable swapRouter;
//     /// @notice Address to receive protocol fees.
//     address public immutable protocolFeeRecipient;
//     /// @notice Address of the Cult rewards contract.
//     address public immutable cultRewards;

//     /// @notice Address of the Uniswap V3 pool.
//     address public poolAddress;
//     /// @notice Address of the token creator.
//     address public tokenCreator;
//     /// @notice Token ID for the liquidity position in Uniswap V3.
//     uint256 public lpTokenId;
//     /// @notice URI for the ERC20 token metadata.
//     string public tokenURI;
//     /// @notice Merkle root for airdrop verification.
//     bytes32[] public merkleRoots;
//     /// @notice Current market type (BONDING_CURVE or UNISWAP_POOL).
//     MarketType public marketType;

//     /// ==================== Constructor ==================== ///
//     /// @notice Constructor to set immutable addresses.
//     /// @param _protocolFeeRecipient Address to receive protocol fees.
//     /// @param _cultRewards Address of the Cult rewards contract.
//     /// @param _weth Address of the WETH token.
//     /// @param _nonfungiblePositionManager Address of the Uniswap V3 position manager.
//     /// @param _swapRouter Address of the Uniswap V3 swap router.
//     constructor(
//         address _protocolFeeRecipient,
//         address _cultRewards,
//         address _weth,
//         address _nonfungiblePositionManager,
//         address _swapRouter
//     ) {
//         if (_protocolFeeRecipient == address(0)) revert AddressZero();
//         if (_cultRewards == address(0)) revert AddressZero();
//         if (_weth == address(0)) revert AddressZero();
//         if (_nonfungiblePositionManager == address(0)) revert AddressZero();
//         if (_swapRouter == address(0)) revert AddressZero();

//         protocolFeeRecipient = _protocolFeeRecipient;
//         cultRewards = _cultRewards;
//         WETH = _weth;
//         nonfungiblePositionManager = _nonfungiblePositionManager;
//         swapRouter = _swapRouter;
//     }

//     /// ==================== Public Functions ==================== ///

//     /// ==================== Initializer ==================== ///
//     /// @notice Initializes a new Cult token.
//     /// @param _tokenCreator The address of the token creator.
//     /// @param _tokenURI The ERC20 token URI.
//     /// @param _name The token name.
//     /// @param _symbol The token symbol.
//     /// @param _merkleRoots The merkle roots for airdrop verification.
//     /// @param airdropAmount The amount of tokens to airdrop.
//     /// @param airdropContract The address of the airdrop contract.
//     function initialize(
//         address _tokenCreator,
//         string memory _tokenURI,
//         string memory _name,
//         string memory _symbol,
//         bytes32[] calldata _merkleRoots,
//         uint256 airdropAmount,
//         address airdropContract
//     ) public payable initializer {
//         // Set the merkle roots
//         merkleRoots = _merkleRoots;

//         // Validate the creation parameters
//         if (_tokenCreator == address(0)) revert AddressZero();

//         // Initialize base contract state
//         __ERC20_init(_name, _symbol);
//         __ReentrancyGuard_init();

//         // Initialize token and market state
//         marketType = MarketType.UNISWAP_POOL;
//         tokenCreator = _tokenCreator;
//         tokenURI = _tokenURI;

//         //Minting to airdrop contract, similar to buy operation
//         //_mint(airdropContract, airdropAmount);
//         _mint(airdropContract, airdropAmount);
//         AIRDROPPED_SUPPLY = airdropAmount;
//     }

//     /// @notice Purchases tokens using ETH, either from the bonding curve or Uniswap V3 pool.
//     /// @param minOrderSize The minimum tokens to prevent slippage.
//     /// @param sqrtPriceLimitX96 The price limit for Uniswap V3 pool swaps, ignored if market is bonding curve.
//     function buy(
//         uint256 minOrderSize,
//         uint160 sqrtPriceLimitX96
//     ) public payable nonReentrant {
//         // Calculate the remaining ETH
//         uint256 totalCost = msg.value;

//         // Convert the ETH to WETH and approve the swap router
//         IWETH(WETH).deposit{value: totalCost}();
//         IWETH(WETH).approve(swapRouter, totalCost);

//         // Set up the swap parameters
//         ISwapRouter.ExactInputSingleParams memory params = ISwapRouter.ExactInputSingleParams({
//             tokenIn: WETH,
//             tokenOut: address(this),
//             fee: LP_FEE,
//             recipient: msg.sender,
//             amountIn: totalCost,
//             amountOutMinimum: minOrderSize,
//             sqrtPriceLimitX96: sqrtPriceLimitX96
//         });

//         ISwapRouter(swapRouter).exactInputSingle(params);
        
//     }

//     /// @notice Sells tokens for ETH on Uniswap V3 pool.
//     /// @param tokensToSell The number of tokens to sell.
//     /// @param minPayoutSize The minimum ETH payout to prevent slippage.
//     /// @param sqrtPriceLimitX96 The price limit for Uniswap V3 pool swaps, ignored if market is bonding curve.
//     function sell(
//         uint256 tokensToSell,
//         uint256 minPayoutSize,
//         uint160 sqrtPriceLimitX96
//     ) external nonReentrant {
//         // Ensure the sender has enough liquidity to sell
//         if (tokensToSell > balanceOf(msg.sender)) {
//             revert InsufficientLiquidity();
//         }

//         transfer(address(this), tokensToSell);
//         this.approve(swapRouter, tokensToSell);

//         // Set up the swap parameters
//         ISwapRouter.ExactInputSingleParams memory params = ISwapRouter.ExactInputSingleParams({
//             tokenIn: address(this),
//             tokenOut: WETH,
//             fee: LP_FEE,
//             recipient: address(this),
//             amountIn: tokensToSell,
//             amountOutMinimum: minPayoutSize,
//             sqrtPriceLimitX96: sqrtPriceLimitX96
//         });

//         // Execute the swap
//         uint256 payout = ISwapRouter(swapRouter).exactInputSingle(params);

//         // Withdraw the ETH from the contract
//         IWETH(WETH).withdraw(payout);

//         // Send the payout to the recipient
//         (bool success,) = msg.sender.call{value: payout}("");
//         if (!success) revert EthTransferFailed();
//     }


//     /// @notice Transfers tokens to a specified address.
//     /// @param to The address to transfer to.
//     /// @param value The amount to be transferred.
//     /// @return A boolean indicating success.
//     function transfer(address to, uint256 value) public override(ERC20Upgradeable) returns (bool) {
//         return super.transfer(to, value); // Calls the ERC20 transfer
//     }

//     /// @notice Burns tokens after the market has graduated to Uniswap V3.
//     /// @param tokensToBurn The number of tokens to burn.
//     function burn(uint256 tokensToBurn) external {
//         _burn(msg.sender, tokensToBurn);
//     }

//     /// @notice receives ETH so it can graduate manually
//     receive() external payable {
//         if (msg.sender == WETH) {
//             return;
//         }
//         //revert("Not allowed");
//         //buy(msg.sender, msg.sender, address(0), "", marketType, 0, 0);
//     }

//     /// @dev For receiving the Uniswap V3 LP NFT on market graduation.
//     /// @return The selector to confirm the token transfer.
//     function onERC721Received(address, address, uint256, bytes calldata) external view returns (bytes4) {
//         if (msg.sender != poolAddress) revert OnlyPool();

//         return this.onERC721Received.selector;
//     }

//     //if anything fails, send all back to tokenCreator
//     function retrieveTokensETH() external nonReentrant {
//         if (msg.sender != tokenCreator) revert OnlyTokenCreator();

//         // Send all Cult held by contract
//         uint256 cultHeld = balanceOf(address(this));
//         if (cultHeld > 0) {
//             _transfer(address(this), tokenCreator, cultHeld);
//         }

//         // If total supply hasn't hit cap, mint remaining and send
//         uint256 currentTotal = totalSupply();
//         if (currentTotal < MAX_TOTAL_SUPPLY) {
//             uint256 remaining = MAX_TOTAL_SUPPLY - currentTotal;
//             _mint(tokenCreator, remaining);
//         }

//         // Send back all ETH: unwrap any WETH first
//         uint256 wethBal = IERC20(WETH).balanceOf(address(this));
//         if (wethBal > 0) {
//             // pull WETH back to ETH
//             IWETH(WETH).withdraw(wethBal);
//         }

//         uint256 ethBal = address(this).balance;
//         if (ethBal > 0) {
//             (bool sentEth,) = tokenCreator.call{value: ethBal}("");
//             if (!sentEth) revert EthTransferFailed();
//         }
//     }

//         /// @dev No-op to allow a swap on the pool to set the correct initial price, if needed.
//     /// @param amount0Delta The change in token0 balance of the pool.
//     /// @param amount1Delta The change in token1 balance of the pool.
//     /// @param data Additional data with no specified format.
//     function uniswapV3SwapCallback(int256 amount0Delta, int256 amount1Delta, bytes calldata data) external {}


//     /// @notice Bootstraps a full-range Uniswap V3 pool at a fixed price.
//     /// @dev Only the `tokenCreator` can call this once while in BONDING_CURVE.
//     function graduateMarket(
//         uint160 _POOL_SQRT_PRICE_X96_WETH_0,
//         uint160 _POOL_SQRT_PRICE_X96_TOKEN_0
//     ) external returns (address) {
//         if (msg.sender != tokenCreator) revert OnlyTokenCreator();

//         // 1.  Cache the chosen sqrt-price based on token ordering
//         address token0 = WETH < address(this) ? WETH : address(this);
//         address token1 = WETH < address(this) ? address(this) : WETH;

//         POOL_SQRT_PRICE_X96_WETH_0  = _POOL_SQRT_PRICE_X96_WETH_0;
//         POOL_SQRT_PRICE_X96_TOKEN_0 = _POOL_SQRT_PRICE_X96_TOKEN_0;

//         uint160 sqrtPriceX96 = token0 == WETH
//             ? POOL_SQRT_PRICE_X96_WETH_0
//             : POOL_SQRT_PRICE_X96_TOKEN_0;

//         // 2.  Create & initialise the pool if it doesn’t exist
//         poolAddress = INonfungiblePositionManager(nonfungiblePositionManager)
//             .createAndInitializePoolIfNecessary(
//                 token0,
//                 token1,
//                 LP_FEE,
//                 sqrtPriceX96
//             );

//         // 3.  Do the heavy lifting (deposit, mint LP, refund / burn)
//         _graduateMarket();

//         return poolAddress;
//     }

//     function _graduateMarket() internal {
//         /* ---- 0. Wrap contract ETH → WETH and mint remaining supply ------------ */
//         uint256 ethLiquidity = address(this).balance;
//         if (ethLiquidity == 0) revert("No ETH to bootstrap");
//         IWETH(WETH).deposit{value: ethLiquidity}();

//         SECONDARY_MARKET_SUPPLY = MAX_TOTAL_SUPPLY - totalSupply();
//         _mint(address(this), SECONDARY_MARKET_SUPPLY);

//         /* ---- 1. Resolve ordering & price -------------------------------------- */
//         bool wethIsToken0 = address(WETH) < address(this);
//         uint160 desiredSqrtPrice = wethIsToken0
//             ? POOL_SQRT_PRICE_X96_WETH_0
//             : POOL_SQRT_PRICE_X96_TOKEN_0;

//         // CULT per WETH (18-dec) when WETH is token0
//         uint256 price1e18 = _price1e18(desiredSqrtPrice);
//         // WETH per CULT (18-dec) for the inverse case
//         uint256 invPrice1e18 = FullMath.mulDiv(1e18, 1e18, price1e18);

//         /* ---- 2. Fetch balances ------------------------------------------------- */
//         uint256 wethBal = IERC20(WETH).balanceOf(address(this));
//         uint256 cultBal = balanceOf(address(this));

//         uint256 wethUsed;
//         uint256 cultUsed;

//         /* ---- 3. Pick limiting asset, refund / burn excess ---------------------- */
//         if (wethIsToken0) {
//             uint256 cultNeeded = FullMath.mulDiv(wethBal, price1e18, 1e18);
//             if (cultNeeded <= cultBal) {
//                 // ETH-limited
//                 wethUsed = wethBal;
//                 cultUsed = cultNeeded;
//                 _burn(address(this), cultBal - cultNeeded);
//             } else {
//                 // CULT-limited
//                 cultUsed = cultBal;
//                 wethUsed = FullMath.mulDiv(cultBal, 1e18, price1e18);
//                 _refundWeth(tokenCreator, wethBal - wethUsed);
//             }
//         } else {
//             uint256 wethNeeded = FullMath.mulDiv(cultBal, invPrice1e18, 1e18);
//             if (wethNeeded <= wethBal) {
//                 // CULT-limited
//                 cultUsed = cultBal;
//                 wethUsed = wethNeeded;
//                 _refundWeth(tokenCreator, wethBal - wethNeeded);
//             } else {
//                 // ETH-limited
//                 wethUsed = wethBal;
//                 cultUsed = FullMath.mulDiv(wethBal, 1e18, invPrice1e18);
//                 _burn(address(this), cultBal - cultUsed);
//             }
//         }
//         require(wethUsed > 0 && cultUsed > 0, "Nothing to add");

//         /* ---- 4. Approvals for exact spend ------------------------------------- */
//         IERC20(WETH).approve(nonfungiblePositionManager, wethUsed);
//         IERC20(address(this)).approve(nonfungiblePositionManager, cultUsed);

//         /* ---- 5. Mint full-range LP NFT ---------------------------------------- */
//         INonfungiblePositionManager.MintParams memory params = (
//             INonfungiblePositionManager.MintParams({
//                 token0: wethIsToken0 ? WETH : address(this),
//                 token1: wethIsToken0 ? address(this): WETH,
//                 fee: LP_FEE,
//                 tickLower: LP_TICK_LOWER,
//                 tickUpper: LP_TICK_UPPER,
//                 amount0Desired: wethIsToken0 ? wethUsed : cultUsed,
//                 amount1Desired: wethIsToken0 ? cultUsed : wethUsed,
//                 amount0Min: 0,
//                 amount1Min: 0,
//                 recipient: tokenCreator,          // LP NFT → creator
//                 deadline: block.timestamp
//             })
//         );

//         (lpTokenId,,,) =
//             INonfungiblePositionManager(nonfungiblePositionManager).mint(params);

//         emit CultMarketGraduated(
//             address(this),
//             poolAddress,
//             wethUsed,
//             cultUsed,
//             lpTokenId,
//             marketType
//         );
//     }
//     /// ==================== Internal Functions ==================== ///

//     // Define the ERC20 storage location using the same slot as ERC20Upgradeable
//     // keccak256(abi.encode(uint256(keccak256("openzeppelin.storage.ERC20")) - 1)) & ~bytes32(uint256(0xff))
//     bytes32 private constant ERC20_STORAGE_LOCATION = 0x52c63247e1f47db19d5ce0460030c497f067ca4cebf71ba98eeadabe20bace00;

//     function _getERC20StorageInternal() private pure returns (ERC20Upgradeable.ERC20Storage storage $) {
//         assembly {
//             $.slot := ERC20_STORAGE_LOCATION
//         }
//     }

//     // @dev Overrides ERC20's _update function to
//     //      - Prevent transfers to the pool if the market has not graduated.
//     //      - Emit the superset `CultTokenTransfer` event with each ERC20 transfer.
//     // @param from The address from which tokens are transferred.
//     // @param to The address to which tokens are transferred.
//     // @param value The amount of tokens transferred.
//     function _update(address from, address to, uint256 value) internal virtual override {
//         if (marketType == MarketType.BONDING_CURVE && to == poolAddress) {
//             revert MarketNotGraduated();
//         }

//         super._update(from, to, value);

//         emit CultTokenTransfer(from, to, value, balanceOf(from), balanceOf(to), totalSupply());
//     }

//     function _price1e18(uint160 sqrtPriceX96)
//         internal
//         pure
//         returns (uint256)
//     {
//         // (sqrtP² * 1e18) / 2¹⁹²  — overflow-safe
//         return FullMath.mulDiv(
//             uint256(sqrtPriceX96) * uint256(sqrtPriceX96),
//             1e18,
//             1 << 192
//         );
//     }
//     function _refundWeth(address to, uint256 amount) internal {
//         if (amount == 0) return;
//         IWETH(WETH).withdraw(amount);
//         (bool ok,) = to.call{value: amount}("");
//         if (!ok) revert EthTransferFailed();
//     }

// }
