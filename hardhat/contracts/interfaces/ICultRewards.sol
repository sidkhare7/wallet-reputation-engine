// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface ICultRewards {
    error ARRAY_LENGTH_MISMATCH();
    error ADDRESS_ZERO();
    error INVALID_DEPOSIT();
    error INVALID_WITHDRAW();
    error TRANSFER_FAILED();
    error SIGNATURE_DEADLINE_EXPIRED();
    error INVALID_SIGNATURE();

    event Deposit(
        address depositor,
        address to,
        bytes4 reason,
        uint256 value,
        string comment
    );
    event Withdraw(address owner, address to, uint256 amount);

    function balanceOf(address account) external view returns (uint256);

    function deposit(
        address to,
        bytes4 why,
        string calldata comment
    ) external payable;

    function depositBatch(
        address[] calldata recipients,
        uint256[] calldata amounts,
        bytes4[] calldata reasons,
        string calldata comment
    ) external payable;

    function withdrawFor(address to, uint256 amount) external;
}
