import { ethers, upgrades, network, run } from "hardhat";
import { networkConfig, testNetworkChains } from "../helper-hardhat-config";
import { updateContractsJson } from "../utils/updateContracts";

export const deploySwapRouter = async () => {
  const accounts = await ethers.getSigners();
  const networkName = network.name;
  const owner = accounts[0].address;
  
  // Get deployer address from config or use the owner address
  let deployer = networkConfig[networkName].deployer;
  if (deployer === "DYNAMIC") {
    deployer = owner;
  }
  
  console.log(`Network: ${networkName}`);
  console.log(`Owner: ${owner}`);
  console.log(`Deployer: ${deployer}`);

  if (deployer?.toLowerCase() !== owner.toLowerCase()) {
    throw Error("Deployer must be the Owner");
  }
  
  const startBlock: any = await ethers.provider.getBlock("latest");
  console.log(`Start Block: ${startBlock!.number}`);

  console.log("Deploying SwapRouter...");
  const weth = "0x760AfE86e5de5fa0Ee542fc7B7B713e1c5425701"; // WMON
  const swapRouterAddress = "0x4c4eabd5fb1d1a7234a48692551eaecff8194ca7";

  const SwapRouter = await ethers.getContractFactory("UniswapRouter");
  const swapRouterContract = await upgrades.deployProxy(SwapRouter, [swapRouterAddress, weth]);
  await swapRouterContract.waitForDeployment();
  console.log("SwapRouter deployed:", swapRouterContract.target);
  let contracts = [
    { name: "SwapRouter", address: swapRouterContract.target },
  ];
  console.log("Updating contracts JSON...");
  updateContractsJson(contracts);
  console.table(contracts);

  console.log("🚀🚀🚀 SwapRouter Deployment Successful 🚀🚀🚀");
  console.log("\nDeployed Contract Addresses:");
  console.log("============================");
  contracts.forEach(contract => {
    console.log(`${contract.name}: ${contract.address}`);
  });
  console.log("SwapRouter deployed successfully");
};



deploySwapRouter()