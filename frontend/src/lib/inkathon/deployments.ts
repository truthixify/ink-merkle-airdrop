import { contracts } from "@polkadot-api/descriptors"
import * as erc20Dev from "contracts/deployments/erc20/dev"
// import * as merkleAirdropPop from "contracts/deployments/merkle_airdrop/pop"
// import * as erc20Pop from "contracts/deployments/erc20/pop"
import * as erc20Passethub from "contracts/deployments/erc20/passetHub"
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
  contract: contracts.erc20,
  evmAddresses: {
    dev: erc20Dev.evmAddress,
    // pop: erc20Pop.evmAddress,
    passethub: erc20Passethub.evmAddress,
    // Add more deployments here
  },
  ss58Addresses: {
    dev: erc20Dev.ss58Address,
    // pop: erc20Pop.ss58Address,
    passethub: erc20Passethub.ss58Address,
    // Add more deployments here
  },
}

export const deployments = {
  merkleAirdropDev,
  erc20Dev,
  erc20Passethub,
  merkleAirdropPassethub,
  // merkleAirdropPop,
  // erc20Pop,
  // Add more contracts here
}
