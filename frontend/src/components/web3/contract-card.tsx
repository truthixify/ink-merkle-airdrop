import { createReviveSdk, type ReviveSdkTypedApi } from "@polkadot-api/sdk-ink"
import { useChainId, useTypedApi } from "@reactive-dot/react"
import { Loader2 } from "lucide-react"
import { type FixedSizeArray, FixedSizeBinary } from "polkadot-api"
import { useCallback, useEffect, useState } from "react"
import { toast } from "sonner"
import { useSignerAndAddress } from "@/hooks/use-signer-and-address"
import { merkleAirdrop } from "@/lib/inkathon/deployments"
import { ellipsify } from "@/lib/utils"
import { CardSkeleton } from "../layout/skeletons"
import { Button } from "../ui/button-extended"
import { Card, CardHeader, CardTitle } from "../ui/card"
import { Table, TableBody, TableCell, TableRow } from "../ui/table"

interface MerklAirdropState {
  erc20_address: string | null
  root: string | null
  is_claimed: boolean | null
}

interface ClaimArgs {
  value: FixedSizeArray<4, bigint>
  proof: FixedSizeBinary<32>[]
  index: bigint
}

// Example setup values for deployment and testing
const setup = {
  alice_account: "0x9621dde636de098b43efb0fa9b61facfe328f99d",
  bob_account: "0x41dccbd49b26c50d34355ed86ff0fa9e489d1e01",
  airdrop_amount_alice: 100000000n,
  airdrop_amount_bob: 500000000n,
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

export function ContractCard() {
  const [queryIsLoading, setQueryIsLoading] = useState(true)

  const api = useTypedApi()
  const chain = useChainId()
  const { signer, signerAddress } = useSignerAndAddress()

  /**
   * Contract Read (Query)
   */
  const [merkleAirdropState, setmerkleAirdropState] = useState<MerklAirdropState | null>()

  const queryContract = useCallback(async () => {
    setQueryIsLoading(true)
    try {
      if (!api || !chain) return

      // Create SDK & contract instance
      const sdk = createReviveSdk(api as ReviveSdkTypedApi, merkleAirdrop.contract)
      const contract = sdk.getContract(merkleAirdrop.evmAddresses.passethub)

      const erc20AddressResult = await contract.query("erc20_address", {
        origin: signerAddress as string,
      })
      const erc20Address = erc20AddressResult.success
        ? erc20AddressResult.value.response.asHex()
        : null

      const isClaimedResult = await contract.query("is_claimed", {
        origin: signerAddress as string,
        data: {
          recipient: FixedSizeBinary.fromText(signerAddress as string),
        },
      })

      const isClaimed = isClaimedResult.success ? isClaimedResult.value.response : null

      const rootResult = await contract.query("root", {
        origin: signerAddress as string,
      })
      const root = rootResult.success ? rootResult.value.response.asHex() : null

      setmerkleAirdropState({
        erc20_address: erc20Address,
        root: root,
        is_claimed: isClaimed || false,
      })
    } catch (error) {
      console.error(error)
    } finally {
      setQueryIsLoading(false)
    }
  }, [api, chain])

  useEffect(() => {
    queryContract()
  }, [queryContract, signerAddress])

  /**
   * Contract Write (Transaction)
   */
  const handleClaim = useCallback(async () => {
    if (!api || !chain || !signer) return

    const sdk = createReviveSdk(api as ReviveSdkTypedApi, merkleAirdrop.contract)
    const contract = sdk.getContract(merkleAirdrop.evmAddresses.passethub)

    // Map account if not mapped
    const isMapped = await sdk.addressIsMapped(signerAddress)
    if (!isMapped) {
      toast.error("Account not mapped. Please map your account first.")
      return
    }

    // 1. Define the arguments for the contract call
    const callData = {
      value: [setup.airdrop_amount_alice, 0n, 0n, 0n] as FixedSizeArray<4, bigint>,
      proof: [
        FixedSizeBinary.fromArray(setup.proof_for_alice[0] as number[] & { length: 32 }),
      ] as FixedSizeBinary<32>[],
      index: 0n,
    }

    // 2. Use `query` to perform a dry-run and get gas estimation
    const queryResult = await contract.query("claim", {
      origin: signerAddress,
      data: callData,
    })
    console.log(queryResult)

    if (!queryResult.success) {
      toast.error("Failed to estimate gas for the transaction.")
      console.error("Gas estimation error:", queryResult.value)
      return
    }

    // 3. Construct the final transaction arguments using the query result
    // The query result contains gasRequired and storageDepositLimit
    const txArgs = {
      data: callData,
      gasLimit: queryResult.value.gasRequired,
      storageDepositLimit: queryResult.value.storageDeposit,
    }

    // Send transaction
    const tx = contract
      .send("claim", txArgs)
      .signAndSubmit(signer)
      .then((tx) => {
        queryContract()
        if (!tx.ok) throw new Error("Failed to send transaction", { cause: tx.dispatchError })
      })

    toast.promise(tx, {
      loading: "Sending transaction...",
      success: "Successfully flipped",
      error: "Failed to send transaction",
    })
  }, [signer, api, chain])

  if (queryIsLoading) return <CardSkeleton />

  return (
    <Card className="inkathon-card">
      <CardHeader className="relative">
        <CardTitle>Merkle Airdrop Contract</CardTitle>

        <Button
          variant="default"
          size="sm"
          className="-top-2 absolute right-6"
          onClick={() => handleClaim()}
          disabled={merkleAirdropState?.is_claimed || !signer}
        >
          Claim Airdrop
        </Button>
      </CardHeader>

      <Table className="inkathon-card-table">
        <TableBody>
          <TableRow className="">
            <TableCell>Merkle Root</TableCell>
            <TableCell>
              {merkleAirdropState?.root ? (
                ellipsify(merkleAirdropState?.root)
              ) : (
                <Loader2 className="animate-spin" />
              )}
            </TableCell>
          </TableRow>

          <TableRow>
            <TableCell>Contract Address</TableCell>
            <TableCell>{ellipsify(merkleAirdrop.evmAddresses.passethub)}</TableCell>
          </TableRow>

          <TableRow>
            <TableCell>Token Address</TableCell>
            <TableCell>
              {merkleAirdropState?.erc20_address ? (
                ellipsify(merkleAirdropState?.erc20_address)
              ) : (
                <Loader2 className="animate-spin" />
              )}
            </TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </Card>
  )
}
