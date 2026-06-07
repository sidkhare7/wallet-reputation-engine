import { ethers, network } from "hardhat";

async function main() {
    const [deployer] = await ethers.getSigners();
    console.log(`Network: ${network.name}`);
    console.log(`Deployer: ${deployer.address}`);

    let nonce = await ethers.provider.getTransactionCount(deployer.address);
    console.log(`Current nonce: ${nonce}`);

    // Deploy the UniswapRouter implementation only (no proxy)
    const SwapRouter = await ethers.getContractFactory("UniswapRouter"); // expectd: 0x46eD95A7AB1A8f72961B4BFc3366c7b7A375B8eb
    const impl = await SwapRouter.deploy();
    await impl.waitForDeployment();
    console.log(`Implementation deployed at: ${impl.target}`);

    nonce = await ethers.provider.getTransactionCount(deployer.address);
    console.log(`Current nonce after deploy: ${nonce}`);
}

main().catch(error => {
    console.error(error);
    process.exitCode = 1;
});


