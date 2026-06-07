import { ethers, upgrades, network, run } from "hardhat";
import { networkConfig, testNetworkChains } from "../helper-hardhat-config";
import { updateContractsJson } from "../utils/updateContracts";

export const deployAirdropContract = async () => {
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
  const claimCost = ethers.parseEther("1");

  console.log("Deploying AirdropContract...");
  const AirdropContract = await ethers.getContractFactory("AirdropContract");
  const airdropContract = await upgrades.deployProxy(AirdropContract, [owner, owner, claimCost]);
  await airdropContract.waitForDeployment();
  console.log("AirdropContract deployed:", airdropContract.target);
  let contracts = [
    { name: "AirdropContract", address: airdropContract.target },
  ];
  console.log("Updating contracts JSON...");
  updateContractsJson(contracts);
  console.table(contracts);

  console.log("🚀🚀🚀 AirdropContract Deployment Successful 🚀🚀🚀");
  console.log("\nDeployed Contract Addresses:");
  console.log("============================");
  contracts.forEach(contract => {
    console.log(`${contract.name}: ${contract.address}`);
  });
  console.log("AirdropContract deployed successfully");
};



deployAirdropContract()