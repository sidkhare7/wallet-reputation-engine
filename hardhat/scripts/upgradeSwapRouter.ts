import { ethers, upgrades, network } from "hardhat"
import * as helpers from "@nomicfoundation/hardhat-network-helpers"
import netMap from "../constants/networkMapping.json"
import { forkedChain, networkConfig } from "../helper-hardhat-config"

async function main() {
    let tx, txr, deployer
    const networkName = network.name as keyof typeof netMap
    const contractNames = ["UniswapRouter"]

    if (forkedChain.includes(networkName)) {
        await helpers.mine()
        const provider = ethers.provider
        deployer = new ethers.Wallet(process.env.PRIVATE_KEY_ADMIN!.toString(), provider)
    } else {
        ;[deployer] = await ethers.getSigners()
    }
    console.log(deployer?.address)
    const balance = await ethers.provider.getBalance(deployer.address)
    console.log("Balance of deployer before upgrade", balance)

    for (let i = 0; i < contractNames.length; i++) {
        const contractFactory = await ethers.getContractFactory("UniswapRouter", deployer) // For testing on forked network (localhost)
        // const old = await upgrades.forceImport("0xAE127Bae82FAc99d1e632F4C41d55794f22EEeF5", contractFactory) // 0.001093447441281046 // 979570084670975

        // Upgrade the proxy to use the new implementation
        console.log("Upgrading Contract...")
        await upgrades.upgradeProxy(
            "0xAE127Bae82FAc99d1e632F4C41d55794f22EEeF5",
            contractFactory
        )
        console.log(`${contractNames[i]} Contract upgraded successfully`)
    }
}

main()
    .then(() => process.exit(0))
    .catch((error) => {
        console.error(error)
        process.exit(1)
    })
