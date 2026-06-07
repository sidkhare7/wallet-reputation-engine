import { ethers, network } from "hardhat";
import * as helpers from "@nomicfoundation/hardhat-network-helpers";
import netMap from "../constants/networkMapping.json";
import { forkedChain } from "../helper-hardhat-config";
import { CultFactory } from "../typechain-types";
import CultFactoryABI from "../constants/abis/CultFactory.json";
import { generateMerkleTreeAndProofs } from "../scripts/createMerkleProofs";

const main = async () => {
  let tx, txr, deployer;
  const networkName: any = network.name as keyof typeof netMap;

  if (forkedChain.includes(networkName)) {
    await helpers.mine();
    const provider = ethers.provider;
    deployer = new ethers.Wallet(process.env.PRIVATE_KEY_ADMIN!.toString(), provider);
  } else {
    [deployer] = await ethers.getSigners();
  }

  const cultFactory: CultFactory = new ethers.Contract(
    "0x72c06FFD9015aCd5314C4f550DcA15a2055be9c4",
    CultFactoryABI,
    deployer,
  );

  const addresses = ["0x60187Bc4949eE2F01b507a9F77ad615093f44260", "0x7909bC836c98bE432c43CF58CE9442a6564026aE"];

  const { root, proofs } = await generateMerkleTreeAndProofs(addresses);
  console.log("Merkle Root", root);
  console.log("Merkle proofs", proofs);

  tx = await cultFactory.updateMerkleRoot(root, 2);
  txr = await tx.wait();
  console.log("Merkle Root updated", tx.hash);

  // const balance = await ethers.provider.getBalance(deployer.address);
  // console.log("Balance", balance);
  // tx = await cultFactory.deploy(
  //   deployer.address,
  //   "ipfs://Qmchtxo6xqjASjfLa1SSxfs8HM1NW2ceshjNir1KXSy5Br",
  //   "Cult Token",
  //   "CT",
  //   [root],
  //   50000,
  //   // {
  //   //   value: ethers.parseEther("0.001"),
  //   // }
  // );
  //txr = await tx.wait();
  //console.log("Cult Token deployed", tx.hash);
  //console.log("txr", txr?.logs);
};

main()
  .then(() => process.exit(0))
  .catch((error: any) => {
    console.error(error);
    process.exit(1);
  });
