// SPDX-License-Identifier: MIT
pragma solidity ^0.8.17;

import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import {ICultRewards} from "./interfaces/ICultRewards.sol";

/// @title CultRewards
/// @notice Manager of deposits & withdrawals for protocol rewards
/// @dev Inherits from EIP712 for signature verification
contract CultRewards is ICultRewards, EIP712 {
    /// ==================== State Variables ==================== ///
    /// @notice The EIP-712 typehash for gasless withdraws
    bytes32 public constant WITHDRAW_TYPEHASH =
        keccak256(
            "Withdraw(address from,address to,uint256 amount,uint256 nonce,uint256 deadline)"
        );

    /// @notice An account's balance
    mapping(address => uint256) public balanceOf;

    /// @notice An account's nonce for gasless withdraws
    mapping(address => uint256) public nonces;

    /// ==================== Constructor ==================== ///
    /// @notice Initializes the contract with the EIP712 domain separator
    constructor() payable EIP712("CultRewards", "1") {}

    /// ==================== External Functions ==================== ///
    /// @notice The total amount of ETH held in the contract
    /// @return The balance of the contract in wei
    function totalSupply() external view returns (uint256) {
        return address(this).balance;
    }

    /// @notice Deposit ETH for a recipient with an optional comment
    /// @dev Emits a Deposit event
    /// @param to Address to deposit to
    /// @param reason System reason for deposit (used for indexing)
    /// @param comment Optional comment as reason for deposit
    function deposit(
        address to,
        bytes4 reason,
        string calldata comment
    ) external payable {
        if (to == address(0)) {
            revert ADDRESS_ZERO();
        }

        balanceOf[to] += msg.value;

        emit Deposit(msg.sender, to, reason, msg.value, comment);
    }

    /// @notice Deposit ETH for multiple recipients with an optional comment
    /// @dev Emits a Deposit event for each recipient
    /// @param recipients Array of recipient addresses
    /// @param amounts Array of amounts to deposit for each recipient
    /// @param reasons Array of reasons for each deposit
    /// @param comment Optional comment to include with each deposit
    function depositBatch(
        address[] calldata recipients,
        uint256[] calldata amounts,
        bytes4[] calldata reasons,
        string calldata comment
    ) external payable {
        uint256 numRecipients = recipients.length;

        if (
            numRecipients != amounts.length || numRecipients != reasons.length
        ) {
            revert ARRAY_LENGTH_MISMATCH();
        }

        uint256 expectedTotalValue;

        for (uint256 i; i < numRecipients; ) {
            expectedTotalValue += amounts[i];

            unchecked {
                ++i;
            }
        }

        if (msg.value != expectedTotalValue) {
            revert INVALID_DEPOSIT();
        }

        address currentRecipient;
        uint256 currentAmount;

        for (uint256 i; i < numRecipients; ) {
            currentRecipient = recipients[i];
            currentAmount = amounts[i];

            if (currentRecipient == address(0)) {
                revert ADDRESS_ZERO();
            }

            balanceOf[currentRecipient] += currentAmount;

            emit Deposit(
                msg.sender,
                currentRecipient,
                reasons[i],
                currentAmount,
                comment
            );

            unchecked {
                ++i;
            }
        }
    }

    /// @notice Withdraw protocol rewards
    /// @dev Emits a Withdraw event
    /// @param to Address to send the withdrawn ETH to
    /// @param amount Amount to withdraw (0 for total balance)
    function withdraw(address to, uint256 amount) external {
        if (to == address(0)) {
            revert ADDRESS_ZERO();
        }

        address owner = msg.sender;

        if (amount > balanceOf[owner]) {
            revert INVALID_WITHDRAW();
        }

        if (amount == 0) {
            amount = balanceOf[owner];
        }

        balanceOf[owner] -= amount;

        emit Withdraw(owner, to, amount);

        (bool success, ) = to.call{value: amount}("");

        if (!success) {
            revert TRANSFER_FAILED();
        }
    }

    /// @notice Withdraw rewards on behalf of an address
    /// @dev Emits a Withdraw event
    /// @param to Address to send the withdrawn ETH to
    /// @param amount Amount to withdraw (0 for total balance)
    function withdrawFor(address to, uint256 amount) external {
        if (to == address(0)) {
            revert ADDRESS_ZERO();
        }

        if (amount > balanceOf[to]) {
            revert INVALID_WITHDRAW();
        }

        if (amount == 0) {
            amount = balanceOf[to];
        }

        balanceOf[to] -= amount;

        emit Withdraw(to, to, amount);

        (bool success, ) = to.call{value: amount}("");

        if (!success) {
            revert TRANSFER_FAILED();
        }
    }

    /// @notice Execute a withdraw of protocol rewards via signature
    /// @dev Uses EIP-712 for signature verification and emits a Withdraw event
    /// @param from Address to withdraw from
    /// @param to Address to send the withdrawn ETH to
    /// @param amount Amount to withdraw (0 for total balance)
    /// @param deadline Deadline for the signature to be valid
    /// @param v V component of the signature
    /// @param r R component of the signature
    /// @param s S component of the signature
    function withdrawWithSig(
        address from,
        address to,
        uint256 amount,
        uint256 deadline,
        uint8 v,
        bytes32 r,
        bytes32 s
    ) external {
        if (block.timestamp > deadline) {
            revert SIGNATURE_DEADLINE_EXPIRED();
        }

        bytes32 withdrawHash;

        unchecked {
            withdrawHash = keccak256(
                abi.encode(
                    WITHDRAW_TYPEHASH,
                    from,
                    to,
                    amount,
                    nonces[from]++,
                    deadline
                )
            );
        }

        bytes32 digest = _hashTypedDataV4(withdrawHash);

        address recoveredAddress = ecrecover(digest, v, r, s);

        if (recoveredAddress == address(0) || recoveredAddress != from) {
            revert INVALID_SIGNATURE();
        }

        if (to == address(0)) {
            revert ADDRESS_ZERO();
        }

        if (amount > balanceOf[from]) {
            revert INVALID_WITHDRAW();
        }

        if (amount == 0) {
            amount = balanceOf[from];
        }

        balanceOf[from] -= amount;

        emit Withdraw(from, to, amount);

        (bool success, ) = to.call{value: amount}("");

        if (!success) {
            revert TRANSFER_FAILED();
        }
    }
}
