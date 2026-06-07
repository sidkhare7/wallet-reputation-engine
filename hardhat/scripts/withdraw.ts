import { ethers, network } from "hardhat"
import netMap from "../constants/networkMapping.json"

async function main() {

    const networkName = network.name as keyof typeof netMap
    const signer = new ethers.Wallet(process.env.PRIVATE_KEY_ADMIN!.toString(), ethers.provider)

    const routerAddress = "0xAE127Bae82FAc99d1e632F4C41d55794f22EEeF5"
    if (!routerAddress) {
        throw new Error(`SwapRouter address not found for network ${networkName}. Provide --router <address>.`)
    }
    let balance = await ethers.provider.getBalance(signer.address)
    console.log("Balance of signer before withdraw", balance)


    const router = await ethers.getContractAt("UniswapRouter", routerAddress, signer)

    balance = await ethers.provider.getBalance(routerAddress)
    console.log("Balance of router before withdraw", balance)
    
    const tx = await router.withdraw()
    await tx.wait()
    console.log("Withdrawal successful")

    balance = await ethers.provider.getBalance(routerAddress)
    console.log("Balance of router after withdraw", balance)

    balance = await ethers.provider.getBalance(signer.address)
    console.log("Balance of signer after withdraw", balance)


}

main().catch((err) => {
    console.error(err)
    process.exit(1)
})
