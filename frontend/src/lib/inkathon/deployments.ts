import { contracts } from "@polkadot-api/descriptors"
import * as assetsDev from "contracts/deployments/assets/dev"
// import * as merkleAirdropPop from "contracts/deployments/merkle_airdrop/pop"
// import * as erc20Pop from "contracts/deployments/erc20/pop"
import * as assetsPassethub from "contracts/deployments/assets/passetHub"
import * as merkleAirdropDev from "contracts/deployments/merkle_airdrop/dev"
import * as merkleAirdropPassethub from "contracts/deployments/merkle_airdrop/passetHub"

export const merkleAirdrop = {
  contract: contracts.merkle_airdrop,
  evmAddresses: {
    dev: merkleAirdropDev.evmAddress,
    // pop: merkleAirdropPop.evmAddress,
    passethub: merkleAirdropPassethub.evmAddress,
    // Add more deployments here
  },
  ss58Addresses: {
    dev: merkleAirdropDev.ss58Address,
    // pop: merkleAirdropPop.ss58Address,
    passethub: merkleAirdropPassethub.ss58Address,
    // Add more deployments here
  },
}

export const erc20 = {
  contract: contracts.assets,
  evmAddresses: {
    dev: assetsDev.evmAddress,
    // pop: erc20Pop.evmAddress,
    passethub: assetsPassethub.evmAddress,
    // Add more deployments here
  },
  ss58Addresses: {
    dev: assetsDev.ss58Address,
    // pop: erc20Pop.ss58Address,
    passethub: assetsPassethub.ss58Address,
    // Add more deployments here
  },
}

export const deployments = {
  merkleAirdropDev,
  assetsDev,
  assetsPassethub,
  merkleAirdropPassethub,
  // merkleAirdropPop,
  // erc20Pop,
  // Add more contracts here
}
