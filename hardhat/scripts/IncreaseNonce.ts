import { ethers } from "ethers";

const RPC = "https://mainnet.base.org";
const PRIVATE_KEY_ADMIN = process.env.PRIVATE_KEY_ADMIN!; // EOA that deployed on Monad
const TARGET_NONCE = 357;

async function main() {
  const provider = new ethers.JsonRpcProvider(RPC);
  const wallet = new ethers.Wallet(PRIVATE_KEY_ADMIN, provider);

  let current = await provider.getTransactionCount(wallet.address);
  console.log("Current nonce", current);
//   while (current < TARGET_NONCE) {
//     const tx = await wallet.sendTransaction({ to: wallet.address, value: 0 });
//     await tx.wait();
//     current++;
//     console.log("Nonce increased to", current);
//   }
//   console.log("Nonce reached", current);
//   // Now run your deployProxy (implementation uses 363, proxy uses 364)
}
main();

// export PK=YOUR_PRIVATE_KEY
// export ADDR=0x71AF57DFDE2420426440B5cbCD5aB6695195925B
// for i in {1..363}; do
//   cast send $ADDR --value 0 --rpc-url https://mainnet.base.org --private-key $PK || break
// done