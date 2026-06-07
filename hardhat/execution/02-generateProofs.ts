import { ethers } from "hardhat";
import { generateMerkleTreeAndProofs } from "../scripts/createMerkleProofs";

const main = async () => {
  const addresses = ["0x60187Bc4949eE2F01b507a9F77ad615093f44260", "0xA2A0a7C6C29143bF7F9612B0025CBB61c74bC06d"];

  const merkleRoot = await generateMerkleTreeAndProofs(addresses);
  console.log("Merkle Root:", merkleRoot.root);
  console.log("Proofs for first address:", merkleRoot.proofs);
};

main()
  .then(() => process.exit(0))
  .catch((error: any) => {
    console.error(error);
    process.exit(1);
  });
