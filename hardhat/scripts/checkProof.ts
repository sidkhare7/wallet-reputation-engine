import { ethers } from "hardhat";
import { keccak256 as keccak256Hash } from "ethers";

async function main() {
  // Test data
  const merkleRoots = ["0x43075ae23bb4a0f82950c0f46d161d317f54d1ce3bbfd6aad541ee9fca44a6a5"];
  const merkleProof = [
    "0xae50c04ea0787d0303bb023fa1e4a415d717f28cc288dc5945583b35c7e2257a",
    "0x1c2bd074885f700ce7af91db7bb5392237a551810666eb4ea86454744cb79ed1",
    "0xbef24147588fe369d4ed56755d0a7d5b4e67ffc9080e2472e19ee8296140657d",
    "0x109c7de56af1eb94c373306e7c30729bd54bbd43b0e1d5decccc3ba0047e1653",
    "0xf3edef6ab94d0ac4b376bd3cbc3e9b2a80ae245c1d7af7207d963f0af15a8669",
    "0xabeb275be8659b1abdde7eeaf89d33b64e7921b3d42610a4fad1d7279e0dcf70",
    "0xec0aadf77a77b4086f37a2ea9523093ede3076e92a2986262ad0890ec59b1c38",
    "0x2a77d0521f92af5c4b6daff838b5c2621c59e9595d4ea5bf18f1722df9b9e907",
    "0xe965ca9885f2438fcdc00060b0010609c609e313eee98cd91521ca32105c1fb5",
    "0x90b412bc546a0d64cb660af1657a133ab09be312adcf52a912001a29865e017b",
    "0x51a2c2f14103b3bcaf136e7397e88ba7d93409ac45bd9d9040eed19e93131172",
    "0xb6f3ec9d5da5b85f374c7ad8b879f59c76a70e6381a6ce5af431f54eb1899ab8"
  ];
  const address = "0xdfbEBABE1c04f51870B0872D0Be0BcD362cc27be";
  
  
  // Call the equivalent of canClaim
  const canClaimResult = canClaim(address, merkleProof, merkleRoots);
  
  console.log("✅ Can claim?", canClaimResult);
}

function canClaim(
  recipient: string, 
  merkleProof: string[], 
  merkleRoots: string[],
): boolean {
  
  // Hash the leaf - using solidityPacked instead of encode to match abi.encodePacked
  const node = keccak256Hash(ethers.solidityPacked(["address"], [recipient]));
  
  // Check against all merkle roots
  for (let i = 0; i < merkleRoots.length; i++) {
    if (verifyMerkleProof(node, merkleProof, merkleRoots[i])) {
      return true;
    }
  }
  
  return false;
}

function verifyMerkleProof(leaf: string, proof: string[], root: string): boolean {
  let hash = leaf;

  for (let i = 0; i < proof.length; i++) {
    const proofElement = proof[i];
    
    if (hash < proofElement) {
      hash = keccak256Hash(ethers.concat([hash, proofElement]));
    } else {
      hash = keccak256Hash(ethers.concat([proofElement, hash]));
    }
  }

  return hash === root;
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});