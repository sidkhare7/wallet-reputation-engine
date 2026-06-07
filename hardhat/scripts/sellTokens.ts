import { ethers, network } from "hardhat"
import netMap from "../constants/networkMapping.json"

async function main() {

    const networkName = network.name as keyof typeof netMap
    const signer = new ethers.Wallet(process.env.PRIVATE_KEY_USER!.toString(), ethers.provider)

    const routerAddress = "0xAE127Bae82FAc99d1e632F4C41d55794f22EEeF5"
    if (!routerAddress) {
        throw new Error(`SwapRouter address not found for network ${networkName}. Provide --router <address>.`)
    }

    const router = await ethers.getContractAt("UniswapRouter", routerAddress, signer)
    
    console.log(`Network: ${networkName}`)
    console.log(`Router: ${routerAddress}`)
    console.log(`Signer: ${signer.address}`)
    console.log(`Recipient: ${signer.address}`)
    
    const cultAddress = "0x93C33B999230eE117863a82889Fdb342cd6D5C64"

    const recipient = signer.address
    const amountOutMinimum = 0n;
    const sqrtPriceLimitX96 = 0n;
    const amountIn = ethers.parseUnits("1", 18)

    const cult = await ethers.getContractAt("Cult", cultAddress, signer)
    const approveTx = await cult.approve(routerAddress, amountIn)
    await approveTx.wait()
    const allowance = await cult.allowance(recipient, routerAddress)
    console.log(`Allowance: ${allowance}`)
    console.log(`Approve tx: ${approveTx.hash}`)
    await approveTx.wait()
    
    const recipientBalance = await ethers.provider.getBalance(recipient)
    console.log(`Recipient balance: ${recipientBalance}`)

    const tokenBalance = await cult.balanceOf(recipient)
    console.log(`Token balance: ${tokenBalance}`)

    const tx = await router.sell(cultAddress, recipient, amountIn, amountOutMinimum, sqrtPriceLimitX96)
    console.log(`Submitted tx: ${tx.hash}`)
    const receipt = await tx.wait()
    console.log(`Confirmed in block ${receipt?.blockNumber}`)

    const recipientBalance2 = await ethers.provider.getBalance(recipient)
    console.log(`Recipient balance: ${recipientBalance2}`)

    const tokenBalance2 = await cult.balanceOf(recipient)
    console.log(`Token balance: ${tokenBalance2}`)

}

main().catch((err) => {
    console.error(err)
    process.exit(1)
})
