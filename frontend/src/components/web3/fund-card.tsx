import { createReviveSdk, type ReviveSdkTypedApi } from "@polkadot-api/sdk-ink"
import { useChainId, useTypedApi } from "@reactive-dot/react"
import { type FixedSizeArray, FixedSizeBinary } from "polkadot-api"
import { useCallback, useEffect, useState } from "react"
import { toast } from "sonner"
import { useChainMeta } from "@/hooks/use-chain-meta"
import { useSignerAndAddress } from "@/hooks/use-signer-and-address"
import { erc20, merkleAirdrop } from "@/lib/inkathon/deployments"
import { ellipsify } from "@/lib/utils"
import { CardSkeleton } from "../layout/skeletons"
import { Button } from "../ui/button-extended"
import { Card, CardHeader, CardTitle } from "../ui/card"
import { Table, TableBody, TableCell, TableRow } from "../ui/table"

interface ERC20State {
  address: string | null
  name: string | null
  symbol: string | null
  total_supply: bigint | null
  balance: bigint | null
}

export function ChainInfoCard() {
  const { chainMeta, isLoading } = useChainMeta()

  const [queryIsLoading, setQueryIsLoading] = useState(true)

  const api = useTypedApi()
  const chain = useChainId()
  const { signer, signerAddress } = useSignerAndAddress()

  /**
   * Contract Read (Query)
   */
  const [erc20State, setErc20State] = useState<ERC20State | null>()

  const queryContract = useCallback(async () => {
    setQueryIsLoading(true)
    try {
      if (!api || !chain) return

      // Create SDK & contract instance
      const merkleAirdropSdk = createReviveSdk(api as ReviveSdkTypedApi, merkleAirdrop.contract)
      const merkleAirdropContract = merkleAirdropSdk.getContract(
        merkleAirdrop.evmAddresses.passethub,
      )
      const erc20AddressResult = await merkleAirdropContract.query("erc20_address", {
        origin: signerAddress as string,
      })
      const erc20Address = erc20AddressResult.success
        ? erc20AddressResult.value.response.asHex()
        : null

      const sdk = createReviveSdk(api as ReviveSdkTypedApi, erc20.contract)
      const contract = sdk.getContract(erc20Address as string)

      const erc20BalanceResult = await contract.query("balance_of", {
        origin: signerAddress as string,
        data: {
          owner: FixedSizeBinary.fromText(signerAddress as string),
        },
      })
      const erc20Balance = erc20BalanceResult.success ? erc20BalanceResult.value.response : null

      setErc20State({
        address: erc20Address,
        name: null,
        symbol: null,
        total_supply: null,
        balance: erc20Balance ? erc20Balance.reduce((acc, curr) => BigInt(acc) + curr, 0n) : 0n,
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
  const handleFund = useCallback(async () => {
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
    // const callData = {
    //   amount: [erc20State?.balance, 0n, 0n, 0n] as FixedSizeArray<4, bigint>,
    // }

    // // Use `query` to perform a dry-run and get gas estimation
    // const queryResult = await contract.query("fund", {
    //   origin: signerAddress,
    //   data: callData,
    // })

    // if (!queryResult.success) {
    //   toast.error("Failed to estimate gas for the transaction.")
    //   console.error("Gas estimation error:", queryResult.value)
    //   return
    // }

    // // Construct the final transaction arguments using the query result
    // // The query result contains gasRequired and storageDepositLimit
    // const txArgs = {
    //   data: callData,
    //   gasLimit: queryResult.value.gasRequired,
    //   storageDepositLimit: queryResult.value.storageDeposit,
    // }

    // Send transaction
    const tx = contract
      .send("fund", {
        data: {
          amount: [1n, 0n, 0n, 0n] as FixedSizeArray<4, bigint>,
        },
        origin: signerAddress,
      })
      .signAndSubmit(signer)
      .then((tx) => {
        queryContract()
        console.log(tx)

        // if (!tx.ok) throw new Error("Failed to send transaction", { cause: tx })
      })
      .catch((error) => {
        console.error("Transaction error:", error)
        throw error // Re-throw to be caught by toast.promise
      })

    toast.promise(tx, {
      loading: "Sending transaction...",
      success: "Successfully funded",
      error: "Failed to send transaction",
    })
  }, [signer, api, chain])

  if (isLoading) return <CardSkeleton />

  return (
    <Card className="inkathon-card">
      <CardHeader className="relative">
        <CardTitle>Fund</CardTitle>
        <Button
          variant="default"
          size="sm"
          className="-top-2 absolute right-6"
          onClick={() => handleFund()}
          disabled={!signer}
        >
          Fund
        </Button>
      </CardHeader>

      <Table className="inkathon-card-table">
        <TableBody>
          <TableRow>
            <TableCell>Amount</TableCell>
            <TableCell>
              <input type="number" placeholder="funding amount" />
            </TableCell>
          </TableRow>

          <TableRow>
            <TableCell>Token Address</TableCell>
            <TableCell>{ellipsify(erc20State?.address)}</TableCell>
          </TableRow>

          <TableRow>
            <TableCell>Balance</TableCell>
            <TableCell>{erc20State?.balance}</TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </Card>
  )
}
