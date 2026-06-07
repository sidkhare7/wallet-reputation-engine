//SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {ReentrancyGuardUpgradeable} from "@openzeppelin/contracts-upgradeable/utils/ReentrancyGuardUpgradeable.sol";
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {IERC20} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
import "@openzeppelin/contracts/utils/cryptography/MerkleProof.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/// @title Airdrop Contract
/// @notice This contract facilitates the distribution of tokens via an airdrop mechanism.
/// @dev The contract uses a Merkle tree to verify claims and ensures that each address can claim only once.

contract AirdropContract is Initializable, ReentrancyGuardUpgradeable, OwnableUpgradeable {

    /// @notice Maximum total supply of the Cult token (10B tokens)
    uint256 public constant MAX_TOKEN_TOTAL_SUPPLY = 10_000_000_000 ether;

    /// @dev Represents 100% in basis points, where 100% = 1000000
    uint256 public constant BASIS_POINTS = 1_000_000;

    /// ==================== State variables ==================== ///
    /// @notice The total amount of tokens available for airdrop
    uint256 public totalAirdropAmount;

    /// @notice The token address for this airdrop contract
    address public token;

    /// @notice The address that created the token
    address public tokenCreator;

    /// @notice The merkle root for verifying claims
    bytes32[] public merkleRoots;

    /// @notice Mapping to track if an address has claimed their tokens
    /// @dev When true, the `recipient` has already received their allocation and cannot claim again
    mapping(address => bool) public hasClaimed;

    /// @notice Mapping of addresses that have proven eligibility via Merkle proof
    /// @dev Set by `verifyEligibility`. Used by `claim` and `claimForEligible` to gate token distribution
    mapping(address => bool) public eligibleAddresses;

    /// @notice The number of recipients for the airdrop
    uint256 public totalAirdropRecipientCount;

    /// @notice Mapping to track the number of recipients for each merkle root
    mapping(bytes32 => uint32) public validMerkleRoots;

    /// @notice The cost in ETH required to claim tokens
    uint256 public claimCost;

    /// @notice Whether the airdrop is enabled
    bool public claimEnabled;

    /// ==================== Events ==================== ///
    event MerkleRootAirdropSet(bytes32[] merkleRoots, uint32[] holderCounts);
    event TokensClaimed(address indexed token, address indexed recipient, uint256 amount);
    event AirdropContractInitialized(
        // address indexed token,
        address indexed createdBy,
        // bytes32[] merkleRoots,
        // uint256 amount,
        // uint256 totalAirdropRecipientCount,
        uint256 claimCost
    );
    // event ETHTransferred(address indexed token, uint256 amount);
    event TokenAirdropSet(address indexed token, bytes32[] merkleRoots, uint32 airdropPercent, uint256 totalAirdropAmount);

    /// ==================== Errors ==================== ///
    error InvalidMerkleProof();
    error AlreadyClaimed();
    // error InvalidPercentage();
    // error Unauthorized();
    error InvalidParameters();
    error InvalidMerkleRoot();
    error InvalidTotalAmount();
    error InvalidAirdropRecipientCount();
    error InsufficientETH();
    error ETHTransferFailed();
    // error AddressZero();
    error ClaimDisabled();
    error InvalidAirdropPercentage();
    /// ==================== Initializer ==================== ///
    /// @notice Initializes the airdrop contract with the token address and owner
    // /// @param _token The token address associated with this airdrop
    /// @param _tokenCreator The token creator
    /// @param _owner The owner for this airdrop contract
    // /// @param _merkleRoots The merkle roots for verifying claims
    // /// @param _totalAmount The total amount of tokens available for airdrop
    // /// @param _totalAirdropRecipientCount The number of recipients for the airdrop
    /// @param _claimCost The cost in ETH required to claim tokens
    function initialize(
        // address _token,
        address _tokenCreator,
        address _owner,
        // bytes32[] calldata _merkleRoots,
        // uint256 _totalAmount,
        // uint256 _totalAirdropRecipientCount,
        uint256 _claimCost
    ) external initializer {
        __ReentrancyGuard_init();
        // if (_token == address(0) || _tokenCreator == address(0) || _owner == address(0)) revert InvalidParameters();
        if (_tokenCreator == address(0) || _owner == address(0)) revert InvalidParameters();

        // if (_totalAmount == 0) revert InvalidTotalAmount();

        // if(_totalAirdropRecipientCount == 0) revert InvalidAirdropRecipientCount();

        // merkleRoots = _merkleRoots;
        // totalAirdropAmount = _totalAmount;
        claimCost = _claimCost;

        // emit MerkleRootSet(token, _merkleRoots);

        // token = _token;
        tokenCreator = _tokenCreator;
        __Ownable_init(_owner);
        // totalAirdropRecipientCount = _totalAirdropRecipientCount;
        // emit AirdropContractInitialized(token, _tokenCreator, _merkleRoots, _totalAmount, _totalAirdropRecipientCount, _claimCost);
        // emit AirdropContractInitialized(_tokenCreator, _merkleRoots, _totalAmount, _totalAirdropRecipientCount, _claimCost);
        emit AirdropContractInitialized(_tokenCreator, _claimCost);
    }

    /// ==================== External Functions ==================== ///

    /// @notice Verifies Merkle proof and marks the caller as eligible (does not transfer tokens)
    /// @dev Idempotent: re-verifying keeps the address marked eligible. Reverts if already claimed
    /// @param merkleProof Merkle proof demonstrating inclusion of `msg.sender` in at least one eligible root
    function verifyEligibility(bytes32[] calldata merkleProof) external payable nonReentrant {
        if (msg.value < claimCost) revert InsufficientETH();
        if (totalAirdropRecipientCount == 0) revert InvalidAirdropRecipientCount();
        if (hasClaimed[msg.sender]) revert AlreadyClaimed();

        bytes32 node = keccak256(abi.encodePacked(msg.sender));
        bool validProof = false;
        for (uint256 i = 0; i < merkleRoots.length; i++) {
            if (MerkleProof.verify(merkleProof, merkleRoots[i], node)) {
                validProof = true;
                break; // Exit the loop once a valid proof is found
            }
        }

        if (!validProof) revert InvalidMerkleProof();

        // if (msg.value > 0) {
        //     (bool ethTransferSuccess, ) = payable(token).call{value: msg.value}("");
        //     if (!ethTransferSuccess) revert ETHTransferFailed();
        //     emit ETHTransferred(token, msg.value);
        // }

        eligibleAddresses[msg.sender] = true;
    }

    /// @notice Claims airdrop for the caller if already eligible
    /// @dev Requires `eligibleAddresses[msg.sender] == true` and `msg.value >= claimCost`
    function claim() external payable nonReentrant {
        _executeClaim(msg.sender);
    }

    /// @notice Checks if an address has a valid claim
    /// @param recipient The recipient address
    /// @param merkleProof The proof of inclusion in merkle tree
    /// @return bool True if the claim is valid, false otherwise
    function canClaim(address recipient, bytes32[] calldata merkleProof) external view returns (bool) {
        if (hasClaimed[recipient]) return false;

        bytes32 node = keccak256(abi.encodePacked(recipient));

        for (uint256 i = 0; i < merkleRoots.length; i++) {
            if (MerkleProof.verify(merkleProof, merkleRoots[i], node)) {
                return true;
            }
        }
        
        return false;
    }

    /// @notice Withdraw native ETH held by this contract to `to`.
    /// @dev Pass amount == type(uint256).max to withdraw the full balance.
    function withdrawETH(address to, uint256 amount) external nonReentrant onlyOwner {
        if (to == address(0)) revert InvalidParameters();

        uint256 balance = address(this).balance;
        uint256 amountToSend = amount == type(uint256).max ? balance : amount;
        if (amountToSend > balance) amountToSend = balance;

        (bool success, ) = payable(to).call{value: amountToSend}("");
        if (!success) revert ETHTransferFailed();
    }

    /// @notice Withdraw ERC20 tokens (including WETH) held by this contract to `to`.
    /// @dev Pass amount == type(uint256).max to withdraw the full token balance.
    function withdrawToken(address tokenAddress, address to, uint256 amount) external nonReentrant onlyOwner {
        if (to == address(0) || tokenAddress == address(0)) revert InvalidParameters();

        uint256 balance = IERC20(tokenAddress).balanceOf(address(this));
        uint256 amountToSend = amount == type(uint256).max ? balance : amount;
        if (amountToSend > balance) amountToSend = balance;

        SafeERC20.safeTransfer(IERC20(tokenAddress), to, amountToSend);
    }

    /// @notice Sets the claim enabled flag
    /// @dev Can only be called by the owner
    /// @param _claimEnabled The new claim enabled flag
    function setClaimEnabled(bool _claimEnabled) external onlyOwner {
        claimEnabled = _claimEnabled;
    }

    /// @notice Sets the token address for this airdrop contract
    /// @dev Can only be called by the owner
    /// @param _token The new token address
    /// @param _merkleRoots The new merkle roots
    /// @param _airdropPercent The new airdrop percentage
    function setTokenAirdrop(address _token, bytes32[] memory _merkleRoots, uint32 _airdropPercent) external onlyOwner {
        if (_token == address(0)) revert InvalidParameters();
        token = _token;
        for (uint256 i; i < _merkleRoots.length; i++) {
            if (_merkleRoots[i] == bytes32(0)) revert InvalidMerkleRoot();
            merkleRoots.push(_merkleRoots[i]);
        }
        totalAirdropRecipientCount = _calculateAirdropRecipientCount(_merkleRoots, _airdropPercent);
        totalAirdropAmount = (MAX_TOKEN_TOTAL_SUPPLY / BASIS_POINTS) * _airdropPercent;
        emit TokenAirdropSet(_token, _merkleRoots, _airdropPercent, totalAirdropAmount);
    }

    /// @notice Updates the merkle roots for verifying claims
    /// @dev Can only be called by the owner
    /// @param _merkleRoots The new merkle roots
    /// @param _holderCounts The new holder counts
    function updateMerkleRootsAirdrop(bytes32[] memory _merkleRoots, uint32[] memory _holderCounts) external onlyOwner {
        for (uint256 i = 0; i < _merkleRoots.length; i++) {
            if (_merkleRoots[i] == 0) revert InvalidMerkleRoot();
            validMerkleRoots[_merkleRoots[i]] = _holderCounts[i];
            merkleRoots.push(_merkleRoots[i]);
        }
        emit MerkleRootAirdropSet(_merkleRoots, _holderCounts);
    }

    /// @notice Calculates the number of recipients for the airdrop
    /// @dev Internal function to calculate the number of recipients for the airdrop
    /// @param _merkleRoots The new merkle roots
    /// @param _airdropPercent The new airdrop percentage
    /// @return recipentCount The number of recipients for the airdrop
    function _calculateAirdropRecipientCount(bytes32[] memory _merkleRoots, uint32 _airdropPercent)
        internal
        view
        returns (uint32 recipentCount)
    {
        // / @dev The airdrop percentage is calculated in basis points, where 100% is equivalent to 1,000,000.
        /// Therefore, an airdrop percentage of 10000 represents 1% of the total supply.
        // Max airdrop rn is 10% or 100000
        if (_airdropPercent > 0) {
            if (_airdropPercent < 10000 || _airdropPercent > 1000000) revert InvalidAirdropPercentage();
            if (_merkleRoots.length > 4) revert InvalidMerkleRoot();
            for (uint256 i = 0; i < _merkleRoots.length; i++) {
                uint32 validMerkleRootHolderCount = validMerkleRoots[_merkleRoots[i]];
                if (validMerkleRootHolderCount == 0) {
                    revert InvalidMerkleRoot();
                } else {
                    recipentCount += validMerkleRootHolderCount;
                }
            }
        }
    }

    /// @dev Internal claim execution used by both `claim()` and `claimForEligible(address)`
    /// @param recipient The address to mark as claimed and to receive the token allocation
    function _executeClaim(address recipient) internal {
        if (!claimEnabled) revert ClaimDisabled();
        if (!eligibleAddresses[recipient]) revert InvalidMerkleProof();
        if (hasClaimed[recipient]) revert AlreadyClaimed();
        if (totalAirdropRecipientCount == 0) revert InvalidAirdropRecipientCount();

        uint256 amount = totalAirdropAmount / totalAirdropRecipientCount;

        hasClaimed[recipient] = true;

        SafeERC20.safeTransfer(IERC20(token), recipient, amount);
        emit TokensClaimed(token, recipient, amount);
    }
}
