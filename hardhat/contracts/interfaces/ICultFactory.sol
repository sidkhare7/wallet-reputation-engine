// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;
interface ICultFactory {
    /// @notice Emitted when a new Cult token is created
    /// @param factoryAddress The address of the factory that created the token
    /// @param tokenCreator The address of the creator of the token
    /// @param protocolFeeRecipient The address of the protocol fee recipient
    /// @param tokenURI The URI of the token
    /// @param name The name of the token
    /// @param symbol The symbol of the token
    /// @param tokenAddress The address of the token
    /// @param poolAddress The address of the pool
    /// @param airdropContract The address of the airdrop contract
    event CultTokenCreated(
        address factoryAddress,
        address indexed tokenCreator,
        address protocolFeeRecipient,
        string tokenURI,
        string name,
        string symbol,
        address indexed tokenAddress,
        address indexed poolAddress,
        address airdropContract,
        bytes32[] merkle_roots,
        uint256 total_amount,
        uint256 total_airdrop_recipient_count
    );

    /// @notice Deploys a Cult ERC20 token
    /// @param _tokenCreator The address of the token creator
    /// @param _tokenURI The ERC20z token URI
    /// @param _name The ERC20 token name
    /// @param _symbol The ERC20 token symbol
    // /// @param _merkleRoots The merkle roots of the airdrop
    // /// @param _airdropPercent The percentage of the airdrop
    // /// @param airdropClaimCost The cost of the airdrop claim
    /// @param sqrtPriceX96_WETH_0 The initial square root price of the WETH token
    /// @param sqrtPriceX96_TOKEN_0 The initial square root price of the Cult token
    function deploy(
        address _tokenCreator,
        string memory _tokenURI,
        string memory _name,
        string memory _symbol,
        // bytes32[] calldata _merkleRoots,
        // uint32 _airdropPercent,
        // uint256 airdropClaimCost,
        uint160 sqrtPriceX96_WETH_0,
        uint160 sqrtPriceX96_TOKEN_0
    ) external payable;
}

