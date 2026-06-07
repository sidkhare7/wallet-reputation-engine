// // SPDX-License-Identifier: MIT
// pragma solidity ^0.8.23;
// import {ReentrancyGuardUpgradeable} from "@openzeppelin/contracts-upgradeable/utils/ReentrancyGuardUpgradeable.sol";
// import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
// import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
// import {UUPSUpgradeable} from "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
// import {ICult} from "./interfaces/ICult.sol";
// import {ISwapRouter} from "./interfaces/ISwapRouter.sol";
// import {IWETH} from "./interfaces/IWETH.sol";
// import {IUniswapV3Pool} from "./interfaces/IUniswapV3Pool.sol";
// /**
//  * @title CultV2
//  * @dev Implementation of the Cult token with bonding curve and Uniswap V3 pool functionalities.
//  */
// contract CultV2 is ICult, UUPSUpgradeable, ERC20Upgradeable, ReentrancyGuardUpgradeable, OwnableUpgradeable {
//     /// ==================== Errors ==================== ///
//     error InvalidParameters();
//     error OnlyWETH();

//     /// ==================== Constants ==================== ///
//     /// @notice Maximum total supply of the Cult token (10B tokens)
//     uint256 public constant MAX_TOTAL_SUPPLY = 10_000_000_000e18;
    
//     /// ==================== State variables ==================== ///
//     /// @notice Address of the WETH token.
//     address public WETH;
//     /// @notice Address of the Uniswap V3 swap router.
//     address public swapRouter;
//     /// @notice URI for the ERC20 token metadata.
//     string public tokenURI;
//     /// @notice LP_FEE
//     uint24 internal constant LP_FEE = 3000; // 0.3 %
//     /// @notice Uniswap V3 pool address
//     address public poolAddress;

//     /// @custom:oz-upgrades-unsafe-allow constructor
//     constructor() {
//         _disableInitializers();
//     }

//     /// @notice Initializes the contract
//     /// @param _weth Address of the WETH token
//     /// @param _swapRouter Address of the Uniswap V3 swap router
//     /// @param _tokenURI URI for the ERC20 token metadata
//     /// @param _name ERC20 token name
//     /// @param _symbol ERC20 token symbol
//     function initialize(
//         address _weth,
//         address _swapRouter,
//         string memory _tokenURI,
//         string memory _name,
//         string memory _symbol 
//     ) public initializer {
//         if(_weth == address(0) || _swapRouter == address(0) || bytes(_name).length == 0 || bytes(_symbol).length == 0 || bytes(_tokenURI).length == 0) revert InvalidParameters();
//         WETH = _weth;
//         swapRouter = _swapRouter;
//         __ERC20_init(_name, _symbol);
//         __ReentrancyGuard_init();
//         __Ownable_init(msg.sender);
//         __UUPSUpgradeable_init();
//         tokenURI = _tokenURI;
//         _mint(msg.sender, MAX_TOTAL_SUPPLY);
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

//     /// @notice Sets the Uniswap V3 pool address.
//     /// @param _poolAddress The address of the Uniswap V3 pool.
//     function setPoolAddress(address _poolAddress) external onlyOwner {
//         poolAddress = _poolAddress;
//     }

//     /// @notice Burns tokens after the market has graduated to Uniswap V3.
//     /// @param tokensToBurn The number of tokens to burn.
//     function burn(uint256 tokensToBurn) external {
//         _burn(msg.sender, tokensToBurn);
//     }

//     /// @notice Only accepts ETH from WETH contract
//     receive() external payable {
//         if (msg.sender != WETH) {
//             revert OnlyWETH();
//         }
//     }

//     /// @notice Allows owner to withdraw any ETH that gets stuck
//     function withdrawETH() external onlyOwner {
//         (bool success,) = owner().call{value: address(this).balance}("");
//         if (!success) revert EthTransferFailed();
//     }

//     /// @dev Overrides ERC20's _update function to
//     ///      - Prevent transfers to the pool if the market has not graduated.
//     ///      - Emit the superset CultTokenTransfer event with each ERC20 transfer.
//     /// @param from The address from which tokens are transferred.
//     /// @param to The address to which tokens are transferred.
//     /// @param value The amount of tokens transferred.
//     function _update(address from, address to, uint256 value) internal virtual override {
//         super._update(from, to, value);
//         emit CultTokenTransfer(from, to, value, balanceOf(from), balanceOf(to), totalSupply());
//     }

//     /// @notice Required by the OZ UUPS module
//     function _authorizeUpgrade(address newImplementation) internal override onlyOwner {}

// }