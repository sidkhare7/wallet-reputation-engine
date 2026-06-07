import { ethers, network } from "hardhat";
import { networkConfig } from "../helper-hardhat-config";
import { AirdropContract } from "../typechain-types";
export const setToken = async () => {
  const AirdropAddress = "0x3E8a58721dA5E1F83f14075442d0dc4869B24a4D";
  const tokenAddress = "0x69c29fe56771f295a73Af25cc70edf58aF5954d5";

  const accounts = await ethers.getSigners();
  const networkName = network.name;
  const owner = accounts[0].address;
  
  // Get deployer address from config or use the owner address
  let deployer = networkConfig[networkName].deployer;
  if (deployer === "DYNAMIC") {
    deployer = owner;
  }
  

  // Connect to AirdropContract contract instance at the provided factory address.
  const AirdropContract = await ethers.getContractFactory("AirdropContract");
  const airdropContract: AirdropContract = AirdropContract.attach(AirdropAddress);
  console.log("Connected to AirdropContract at:", AirdropAddress);

  const merkleRoot = "0xd39b22f3d0277d1d8d0d036f3e87dfffc98512eba953a4af5759227d43fd7464"; // should be a valid 0x-prefixed 32-byte hex string
  const airdropPercent = 171300;

  // Call updateMerkleRoot function.
  const tx = await airdropContract.setTokenAirdrop(tokenAddress, [merkleRoot], airdropPercent);
  console.log("Transaction submitted. Waiting for confirmation...");
  const receipt = await tx.wait();
  console.log("Token set successfully in tx:", receipt?.hash);
};

setToken()
  .then(() => process.exit(0))
  .catch(error => {
    console.error(error);
    process.exit(1);
  });
