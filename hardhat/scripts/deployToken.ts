import { ethers, network } from "hardhat";
import { networkConfig } from "../helper-hardhat-config";
// import { updateContractsJson } from "../utils/updateContracts";
import { CultFactory } from "../typechain-types";

export const deployToken = async () => {
  const accounts = await ethers.getSigners();
  const networkName = network.name;
  const owner = accounts[0].address;
  
  // Get deployer address from config or use the owner address
  let deployer = networkConfig[networkName].deployer;
  if (deployer === "DYNAMIC") {
    deployer = owner;
  }

  if (deployer?.toLowerCase() !== owner.toLowerCase()) {
    throw Error("Deployer must be the Owner");
  }
  
  const cultFactoryAddress = "0x2f097EFF891340fc538EC33A9edc060AEE42A8B9";

  const CultFactory = await ethers.getContractFactory("CultFactory");
  const cultFactory: CultFactory = await CultFactory.attach(cultFactoryAddress);
  
  let tx = await cultFactory.deploy(owner, "ipfs://bafkreihjvydslk75l6nt4vznmg2sj2cdfy4hkuwvmszca6js4uvflr3ewe", "MONGANG", "$GANG", "79228162514264337593543950336000", "79228162514264337593543950", { value: ethers.parseEther("0") });
  let txr = await tx.wait();
  
  // Extract token address from transaction logs
  if (txr && txr.logs && txr.logs.length > 0) {
    // Look for the first contract address that's not the factory address
    for (const log of txr.logs) {
      if (log.address !== cultFactoryAddress && log.address !== ethers.ZeroAddress) {
        console.log(log.address);
        return;
      }
    }
  }
  
  // Fallback: print message if no token address found
  console.log("Token address not found in transaction logs");
};



deployToken()