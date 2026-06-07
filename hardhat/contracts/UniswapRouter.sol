// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import {Initializable} from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import {UUPSUpgradeable} from "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import {PausableUpgradeable} from "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import {ReentrancyGuardUpgradeable} from "@openzeppelin/contracts-upgradeable/utils/ReentrancyGuardUpgradeable.sol";
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {ISwapRouter} from "./interfaces/ISwapRouter.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IWETH} from "./interfaces/IWETH.sol";

/**
 * @title UniswapRouter
 * @notice A contract that provides a simplified interface for swapping tokens on Uniswap V3
 * @dev This contract is upgradeable and includes pause functionality for emergency situations
 * @dev Inherits from OpenZeppelin's upgradeable contracts for security and upgradeability
 */
contract UniswapRouter is UUPSUpgradeable, PausableUpgradeable, ReentrancyGuardUpgradeable, OwnableUpgradeable { 
    /// @notice Liquidity provider fee in basis points for the Uniswap V3 pool.
    uint24 internal constant LP_FEE = 10000;
    
    /// @notice The address of the Uniswap V3 SwapRouter contract
    address public swapRouter;
    
    /// @notice The address of the WETH (Wrapped Ether) contract
    address public WETH;
    
    /// ==================== Errors ==================== ///
    
    /// @notice Error thrown when invalid parameters are provided
    error InvalidParameters();
    
    /// @notice Error thrown when a zero address is provided where it's not allowed
    error AddressZero();
    
    /// ==================== Events ==================== ///
    
    /// @notice Emitted when the swap router address is updated
    /// @param swapRouter The new swap router address
    event SwapRouterUpdated(address swapRouter);

    /// @notice Emitted when ETH transfer fails
    error EthTransferFailed();
    
    /// ==================== Constructor ==================== ///
    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() payable {
        _disableInitializers();
    }
    
    /**
     * @notice Initializes the contract with required parameters
     * @param _swapRouter The address of the Uniswap V3 SwapRouter contract
     * @param _weth The address of the WETH (Wrapped Ether) contract
     * @dev This function can only be called once during contract initialization
     */
    function initialize(address _swapRouter, address _weth) public initializer {
        __UUPSUpgradeable_init();
        __Pausable_init();
        __ReentrancyGuard_init();
        __Ownable_init(msg.sender);
        if (_swapRouter == address(0)) revert AddressZero();
        if (_weth == address(0)) revert AddressZero();
        swapRouter = _swapRouter;
        WETH = _weth;
    }

    /// ==================== Admin Functions ==================== ///
    
    /**
     * @notice Pauses the contract, preventing all swaps
     * @dev Only callable by the contract owner
     */
    function pause() public onlyOwner {
        _pause();
    }

    /**
     * @notice Unpauses the contract, allowing swaps to resume
     * @dev Only callable by the contract owner
     */
    function unpause() public onlyOwner {
        _unpause();
    }

    /**
     * @notice Updates the swap router address
     * @param _swapRouter The new swap router address
     * @dev Only callable by the contract owner
     * @dev Emits SwapRouterUpdated event
     */
    function setSwapRouter(address _swapRouter) public onlyOwner {
        if (_swapRouter == address(0)) revert AddressZero();
        swapRouter = _swapRouter;
        emit SwapRouterUpdated(swapRouter);
    }

    /// ==================== External Functions ==================== ///
    
    /**
     * @notice Buys tokens for ETH using Uniswap V3
     * @param tokenOut The address of the token to buy
     * @param recipient The address to receive the purchased tokens
     * @param amountOutMinimum The minimum amount of tokens to receive (slippage protection)
     * @param sqrtPriceLimitX96 The price limit for the swap (0 for no limit)
     * @return The number of tokens purchased
     * @dev This function is payable and accepts ETH
     * @dev The ETH is converted to WETH before the swap
     * @dev Reverts if the contract is paused
     * @dev Reverts if any address parameter is zero
     */
    function buy(
        address tokenOut,
        address recipient,
        uint256 amountOutMinimum,
        uint160 sqrtPriceLimitX96
    ) external payable nonReentrant whenNotPaused returns (uint256) {
        if (tokenOut == address(0)) revert InvalidParameters();
        if (recipient == address(0)) revert AddressZero();

        // Convert the ETH to WETH and approve the swap router
        IWETH(WETH).deposit{value: msg.value}();
        IWETH(WETH).approve(swapRouter, msg.value);

        // Set up the swap parameters
        ISwapRouter.ExactInputSingleParams memory params = ISwapRouter.ExactInputSingleParams({
            tokenIn: WETH,
            tokenOut: tokenOut,
            fee: LP_FEE,
            recipient: recipient,
            amountIn: msg.value,
            amountOutMinimum: amountOutMinimum,
            sqrtPriceLimitX96: sqrtPriceLimitX96
        });

        // Execute the swap
        uint256 trueOrderSize = ISwapRouter(swapRouter).exactInputSingle(params);

        return trueOrderSize;
    }
    
    /**
     * @notice Sells tokens for ETH using Uniswap V3
     * @param tokenIn The address of the token to sell
     * @param recipient The address to receive the ETH payout
     * @param amountIn The amount of tokens to sell
     * @param amountOutMinimum The minimum amount of ETH to receive (slippage protection)
     * @param sqrtPriceLimitX96 The price limit for the swap (0 for no limit)
     * @return The amount of ETH received
     * @dev This function requires the caller to approve tokens to this contract first
     * @dev Reverts if the contract is paused
     * @dev Reverts if any address parameter is zero or amountIn is zero
     */
    function sell(
        address tokenIn,
        address recipient,
        uint256 amountIn,
        uint256 amountOutMinimum,
        uint160 sqrtPriceLimitX96
    ) external nonReentrant whenNotPaused returns (uint256) {
        if (tokenIn == address(0)) revert InvalidParameters();
        if (amountIn == 0) revert InvalidParameters();
        if (recipient == address(0)) revert AddressZero();

        IERC20(tokenIn).transferFrom(msg.sender, address(this), amountIn);
        IERC20(tokenIn).approve(swapRouter, amountIn);

        // Set up the swap parameters
        ISwapRouter.ExactInputSingleParams memory params = ISwapRouter.ExactInputSingleParams({
            tokenIn: tokenIn,
            tokenOut: WETH,
            fee: LP_FEE,
            recipient: address(this),
            amountIn: amountIn,
            amountOutMinimum: amountOutMinimum,
            sqrtPriceLimitX96: sqrtPriceLimitX96
        });

        // Execute the swap
        uint256 truePayoutSize = ISwapRouter(swapRouter).exactInputSingle(params);

        // Withdraw the ETH from the contract
        IWETH(WETH).withdraw(truePayoutSize);

        // Send the payout to the recipient
        (bool success,) = payable(recipient).call{value: truePayoutSize}("");
        if (!success) revert EthTransferFailed();

        return truePayoutSize;
    }

    /**
     * @notice Receives ETH from the WETH contract
     * @dev This function is required for the WETH contract to send ETH to the contract
     */
    receive() external payable {}

    /**
     * @notice Withdraws ETH from the contract
     * @dev This function is only callable by the contract owner
     */
    function withdraw() external onlyOwner {
        (bool success,) = payable(msg.sender).call{value: address(this).balance}("");
        if (!success) revert EthTransferFailed();
    }

    /// ==================== Internal Functions ==================== ///
    
    /**
     * @notice Authorizes the upgrade to a new implementation
     * @param newImplementation The address of the new implementation contract
     * @dev Only callable by the contract owner
     * @dev This function is required for UUPS upgradeability
     */
    function _authorizeUpgrade(address newImplementation) internal override onlyOwner {}
}