import { contracts } from "@polkadot-api/descriptors"
import { FixedSizeBinary } from "polkadot-api"
import { deployContract } from "./utils/deploy-contract"
import { initApi } from "./utils/init-api"
import { writeAddresses } from "./utils/write-addresses"

// Example setup values for deployment and testing
const setup = {
  alice_account: "0x9621dde636de098b43efb0fa9b61facfe328f99d",
  bob_account: "0x41dccbd49b26c50d34355ed86ff0fa9e489d1e01",
  airdrop_amount_alice: 100000000,
  airdrop_amount_bob: 500000000,
  leaf_alice: [
    243, 212, 132, 156, 158, 171, 187, 158, 71, 210, 86, 218, 119, 153, 91, 155, 102, 245, 136, 3,
    42, 91, 254, 10, 114, 160, 149, 114, 134, 1, 41, 53,
  ],
  leaf_bob: [
    124, 127, 31, 178, 235, 57, 38, 213, 225, 150, 117, 3, 6, 232, 6, 175, 169, 115, 178, 235, 17,
    37, 136, 27, 187, 61, 226, 9, 5, 21, 51, 208,
  ],
  total_supply: 1000000000n,
  proof_for_alice: [
    [
      124, 127, 31, 178, 235, 57, 38, 213, 225, 150, 117, 3, 6, 232, 6, 175, 169, 115, 178, 235, 17,
      37, 136, 27, 187, 61, 226, 9, 5, 21, 51, 208,
    ],
  ],
  proof_for_bob: [
    [
      243, 212, 132, 156, 158, 171, 187, 158, 71, 210, 86, 218, 119, 153, 91, 155, 102, 245, 136, 3,
      42, 91, 254, 10, 114, 160, 149, 114, 134, 1, 41, 53,
    ],
  ],
  index_alice: 0,
  index_bob: 1,
  root: [
    45, 178, 195, 134, 122, 225, 172, 30, 120, 210, 109, 225, 33, 65, 92, 176, 105, 208, 145, 202,
    220, 95, 132, 132, 198, 239, 191, 203, 78, 159, 203, 80,
  ],
  creator: "0xe2235a2ffe0354b27a6a1c543be6bf2920ff2134",
}

/**
 * This script initializes the Polkadot API client and deploys the contract
 * using the provided utilities under './utils'.
 *
 * @options
 *  Environment variables:
 *    CHAIN         - Target chain to deploy the contract to (must be initialized with `bunx papi add <chain>`). Default: `dev`
 *    ACCOUNT_URI   - Account to deploy the contract from. If not set, uses `.env.{CHAIN}` or defaults to `//Alice`
 *    DIR           - Directory to write the contract addresses to. Default: `./deployments`
 *
 * @example
 * CHAIN=dev bun run deploy.ts
 */
const main = async () => {
  const initResult = await initApi()
  const erc20CodeHash = "0x668a3df3b0a4f99f9752fc6c27bf3f644d824a81a66a9615959d8fc25dc460df"

  const deployErc20Result = await deployContract(initResult, "erc20", contracts.erc20, "new", {
    total_supply: [setup.total_supply, 0n, 0n, 0n],
  })

  const deployMerkleAirdropResult = await deployContract(
    initResult,
    "merkle_airdrop",
    contracts.merkle_airdrop,
    "new_no_limits",
    {
      erc20_contract_code_hash: FixedSizeBinary.fromHex(erc20CodeHash),
      root: FixedSizeBinary.fromArray(setup.root),
      total_supply: [setup.total_supply, 0n, 0n, 0n],
    },
  )

  await writeAddresses({ merkle_airdrop: deployMerkleAirdropResult })
  await writeAddresses({ erc20: deployErc20Result })
}

main()
  .catch((error) => {
    console.error(error)
    process.exit(1)
  })
  .finally(() => process.exit(0))
