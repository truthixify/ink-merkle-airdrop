// scripts/setup_airdrop.ts
import fs from "node:fs"
import path from "node:path"
import { parse } from "csv-parse/sync"
import { type LeafData, MerkleTree } from "./utils/merkle_tree"

/**
 * Reads and parses the CSV file into LeafData[]
 * @param filePath Path to CSV file
 */
function loadCSV(filePath: string): LeafData[] {
  const raw = fs.readFileSync(filePath, "utf-8")
  const records = parse(raw, {
    columns: true, // first row is header: address,amount
    skip_empty_lines: true,
    trim: true,
  })

  return records.map((row: any) => {
    if (!row.address || !row.amount) {
      throw new Error(`Invalid CSV row: ${JSON.stringify(row)}`)
    }
    return {
      recipient: row.address.startsWith("0x") ? row.address : `0x${row.address}`,
      value: BigInt(row.amount),
    }
  })
}

/**
 * Builds a MerkleTree and returns the setup object.
 * @param csvPath Path to the CSV file
 */
export function setupAirdrop(csvPath: string) {
  const absPath = path.resolve(csvPath)
  const leafData = loadCSV(absPath)

  if (leafData.length === 0) {
    throw new Error("CSV contains no valid entries")
  }

  const tree = new MerkleTree(leafData)

  // compute total supply from all amounts
  const totalSupply = leafData.reduce((acc, leaf) => acc + leaf.value, 0n)

  return {
    leafData,
    tree,
    root: tree.root!,
    totalSupply,
  }
}

// If run directly: build setup and write JSON for later use
if (require.main === module) {
  const csvPath = process.argv[2] || "./data/airdrop/airdrop.csv"
  const setup = setupAirdrop(csvPath)

  const outDir = path.resolve(__dirname, "../data/airdrop")
  fs.mkdirSync(outDir, { recursive: true })
  const outFile = path.join(outDir, "airdrop_setup.json")

  const json = {
    root: `0x${Buffer.from(setup.root).toString("hex")}`,
    totalSupply: setup.totalSupply.toString(),
    leaves: setup.leafData.map((leaf, i) => ({
      recipient: leaf.recipient,
      value: leaf.value.toString(),
      proof: setup.tree.getProof(i).map((p) => `0x${Buffer.from(p).toString("hex")}`),
    })),
  }

  fs.writeFileSync(outFile, JSON.stringify(json, null, 2))
  console.log(`Airdrop setup written to ${outFile}`)
}
