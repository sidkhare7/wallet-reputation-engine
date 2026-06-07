import { ethers, upgrades, network, run } from "hardhat";
import { networkConfig, testNetworkChains } from "../helper-hardhat-config";
import { updateContractsJson } from "../utils/updateContracts";

export const setupContracts = async () => {
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

  console.log("Deploying CultRewards...");
  const CultRewards = await ethers.getContractFactory("CultRewards");
  const cultRewards = await CultRewards.deploy();
  await cultRewards.waitForDeployment();
  console.log("CultRewards deployed:", cultRewards.target);

  // Protocol configuration
  const CULT_RECS = "0xa85367272da042052e05459Ca3390c185DDb55ca"; // CultRecs multisig wallet
  const protocolFeeRecipient = CULT_RECS;
  const protocolRewards = cultRewards.target;
  // const weth = "0x4200000000000000000000000000000000000006";
  // const nonfungiblePositionManager = "0x46A15B0b27311cedF172AB29E4f4766fbE7F4364";
  // const swapRouter = "0x1b81D678ffb9C0263b24A97847620C99d213eB14";

  // Monad Testnet addresses
  // const weth = "0x261D8c5e9742e6f7f1076Fa1F560894524e19cad";
  // const nonfungiblePositionManager = "0x50ff23E9A8D5DAc05744C367c9DDd588D027982B";
  // const swapRouter = "0x201B36B26b816D061fC552B679f8279Db0Fbbc6A";

  // Monad Testnet addresses (Ref: https://docs.monad.xyz/docs/monad-testnet/deployed-contracts)
  // const weth = "0x261D8c5e9742e6f7f1076Fa1F560894524e19cad";
  const weth = "0x760AfE86e5de5fa0Ee542fc7B7B713e1c5425701"; // WMON
  const nonfungiblePositionManager = "0x3dcc735c74f10fe2b9db2bb55c40fbbbf24490f7";
  const swapRouter = "0x4c4eabd5fb1d1a7234a48692551eaecff8194ca7";

  console.log("Deploying Cult Token Implementation...");
  const CultToken = await ethers.getContractFactory("Cult");
  const cultToken = await CultToken.deploy(
    protocolFeeRecipient,
    protocolRewards,
    weth,
    nonfungiblePositionManager,
    swapRouter,
  );
  await cultToken.waitForDeployment();
  console.log("CultToken deployed:", cultToken.target);

  // const BondingCurve = await ethers.getContractFactory("BondingCurve");
  // const bondingCurve = await BondingCurve.deploy();
  // await bondingCurve.waitForDeployment();
  // console.log("BondingCurve deployed:", bondingCurve.target);

  const AirdropContract = await ethers.getContractFactory("AirdropContract");
  const airdropContract = await AirdropContract.deploy();
  await airdropContract.waitForDeployment();
  console.log("AirdropContract deployed:", airdropContract.target);

  console.log("Deploying CultFactory Proxy...");
  const CultFactory = await ethers.getContractFactory("CultFactory");
  const cultFactory = await upgrades.deployProxy(CultFactory, [
    cultToken.target,
    airdropContract.target,
  ]);
  await cultFactory.waitForDeployment();
  console.log("CultFactory deployed:", cultFactory.target);

  let contracts = [
    { name: "Cult", address: cultToken.target },
    { name: "CultRewards", address: cultRewards.target },
    { name: "AirdropContract", address: airdropContract.target },
    { name: "CultFactory", address: cultFactory.target },
    { name: "StartBlock", address: startBlock.number },
  ];

  console.log("Updating contracts JSON...");
  updateContractsJson(contracts);
  console.table(contracts);

  console.log("🚀🚀🚀 Cult Deployment Successful 🚀🚀🚀");
  console.log("\nDeployed Contract Addresses:");
  console.log("============================");
  contracts.forEach(contract => {
    console.log(`${contract.name}: ${contract.address}`);
  });
};

setupContracts()
  .then(() => process.exit(0))
  .catch((error: any) => {
    console.error(error);
    process.exit(1);
  });
