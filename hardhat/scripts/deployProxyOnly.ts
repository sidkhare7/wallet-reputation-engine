import { ethers, network } from "hardhat";

// Usage:
//   BASE_WETH and BASE_UNISWAP_ROUTER must be set for Base deployment
//   IMPLEMENTATION should be a deployed UniswapRouter implementation address
// Example:
//   IMPLEMENTATION=0xImplAddr BASE_WETH=0xWETH BASE_UNISWAP_ROUTER=0xUniRouter \
//   npx hardhat run hardhat/scripts/deployProxyOnly.ts --network base

async function main() {
    const implAddress = "0x46eD95A7AB1A8f72961B4BFc3366c7b7A375B8eb";
    const weth = "0x760AfE86e5de5fa0Ee542fc7B7B713e1c5425701";
    const uniSwapRouter = "0x4c4eabd5fb1d1a7234a48692551eaecff8194ca7";

    if (!implAddress || !weth || !uniSwapRouter) {
        throw new Error("Missing env vars: IMPLEMENTATION, BASE_WETH, BASE_UNISWAP_ROUTER");
    }

    const [deployer] = await ethers.getSigners();
    console.log(`Network: ${network.name}`);
    console.log(`Deployer: ${deployer.address}`);
    console.log(`Using implementation: ${implAddress}`);

    let nonce = await ethers.provider.getTransactionCount(deployer.address);
    console.log(`Current nonce: ${nonce}`);

    const SwapRouter = await ethers.getContractFactory("UniswapRouter");
    // Prepare initializer calldata for initialize(address _swapRouter, address _weth)
    const initData = SwapRouter.interface.encodeFunctionData("initialize", [
        uniSwapRouter,
        weth,
    ]);

    // Manually deploy ERC1967Proxy via local wrapper so Hardhat finds the artifact
    const ProxyFactory = await ethers.getContractFactory("OzERC1967Proxy");
    const proxy = await ProxyFactory.deploy(implAddress, initData);
    await proxy.waitForDeployment();

    console.log(`Proxy deployed at: ${proxy.target}`);
    nonce = await ethers.provider.getTransactionCount(deployer.address);
    console.log(`Current nonce after deploy: ${nonce}`);
}

main().catch(err => {
    console.error(err);
    process.exitCode = 1;
});


