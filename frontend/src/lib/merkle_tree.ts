import { keccak_256 } from "@noble/hashes/sha3"
import { getBytes, toBeHex } from "ethers"
import { FixedSizeBinary } from "polkadot-api"

/**
 * @notice Data structure representing a Merkle tree leaf.
 * @param recipient address (20 bytes).
 * @param value Amount associated with the recipient (uint256, 32 bytes).
 */
export type LeafData = {
  recipient: string
  value: bigint
}

/**
 * @title MerkleTree
 * @notice Utility class for building and verifying Merkle proofs.
 * @dev Uses keccak256 hashing with ABI-encoded (address, uint256) leaves.
 */
export class MerkleTree {
  private leaves: Uint8Array[]
  private tree: Uint8Array[][]
  public root: Uint8Array | null

  /**
   * @notice Constructs a Merkle tree from an array of leaf data.
   * @param leafData Array of `{ recipient, value }` objects.
   */
  constructor(leafData: LeafData[]) {
    this.leaves = leafData.map((data) => MerkleTree.encodeLeaf(data.recipient, data.value))
    this.tree = []
    this.root = null
    this.buildTree()
  }

  /**
   * @notice Encodes a leaf as `keccak256(abi.encodePacked(address, uint256))`.
   * @dev Ensures 20-byte address + 32-byte big-endian value format.
   * @param recipient address.
   * @param value Token amount or balance.
   * @return Encoded and hashed leaf (32 bytes).
   */
  public static encodeLeaf(recipient: string, value: bigint): Uint8Array {
    const addr = getBytes(recipient) // 20 bytes
    const val = getBytes(toBeHex(value, 32)) // 32 bytes
    const encoded = new Uint8Array(addr.length + val.length)
    encoded.set(addr, 0)
    encoded.set(val, addr.length)
    return getBytes(keccak_256(encoded))
  }

  /**
   * @notice Hashes two child nodes into a parent node.
   * @dev Computes `keccak256(left || right)`.
   * @param left Left child hash (32 bytes).
   * @param right Right child hash (32 bytes).
   * @return Parent node hash (32 bytes).
   */
  public static hashPair(left: Uint8Array, right: Uint8Array): Uint8Array {
    const concatenated = new Uint8Array(left.length + right.length)
    concatenated.set(left, 0)
    concatenated.set(right, left.length)
    return getBytes(keccak_256(concatenated))
  }

  /**
   * @notice Builds the Merkle tree from the leaves.
   * @dev Fills the `tree` array and computes the `root`.
   */
  private buildTree(): void {
    if (this.leaves.length === 0) {
      this.root = null
      return
    }

    let currentLevel = this.leaves.slice()
    this.tree.push(currentLevel)

    while (currentLevel.length > 1) {
      const nextLevel: Uint8Array[] = []
      for (let i = 0; i < currentLevel.length; i += 2) {
        const left = currentLevel[i]
        const right = i + 1 < currentLevel.length ? currentLevel[i + 1] : left
        nextLevel.push(MerkleTree.hashPair(left, right))
      }
      this.tree.push(nextLevel)
      currentLevel = nextLevel
    }

    this.root = currentLevel[0]
  }

  /**
   * @notice Returns the Merkle proof for a given leaf index.
   * @dev Proof is ordered bottom → top and consists of sibling nodes.
   * @param index Index of the leaf in the original leaves array.
   * @return Array of sibling hashes forming the Merkle proof.
   */
  public getProof(index: number): Uint8Array[] {
    if (index < 0 || index >= this.leaves.length) {
      throw new Error("Leaf index out of bounds.")
    }

    const proof: Uint8Array[] = []
    let currentIndex = index

    for (let level = 0; level < this.tree.length - 1; level++) {
      const currentLevel = this.tree[level]
      const siblingIndex = currentIndex % 2 === 0 ? currentIndex + 1 : currentIndex - 1

      if (siblingIndex < currentLevel.length) {
        proof.push(currentLevel[siblingIndex])
      } else {
        proof.push(currentLevel[currentIndex]) // duplicate if odd
      }
      currentIndex = Math.floor(currentIndex / 2)
    }

    return proof
  }

  /**
   * @notice Verifies a Merkle proof for a leaf.
   * @dev Recomputes the root from the leaf, proof, and index.
   * @param leaf Leaf hash (32 bytes).
   * @param proof Array of sibling hashes.
   * @param index Leaf index in the tree.
   * @param root Expected Merkle root.
   * @return True if the proof is valid and recomputed root matches.
   */
  public static verifyProof(
    leaf: Uint8Array,
    proof: Uint8Array[],
    index: number,
    root: Uint8Array,
  ): boolean {
    let computed = leaf
    let idx = index

    for (const sibling of proof) {
      if (idx % 2 === 0) {
        computed = MerkleTree.hashPair(computed, sibling)
      } else {
        computed = MerkleTree.hashPair(sibling, computed)
      }
      idx = Math.floor(idx / 2)
    }

    return MerkleTree.bytesEqual(computed, root)
  }

  /**
   * @notice Checks equality of two byte arrays.
   * @dev Used internally to compare computed root with expected root.
   * @param a First byte array.
   * @param b Second byte array.
   * @return True if arrays are identical.
   */
  private static bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
    if (a.length !== b.length) return false
    for (let i = 0; i < a.length; i++) {
      if (a[i] !== b[i]) return false
    }
    return true
  }
}

const setup = {
  alice_account: "0x9621dde636de098b43efb0fa9b61facfe328f99d",
  bob_account: "0x41dccbd49b26c50d34355ed86ff0fa9e489d1e01",
  airdrop_amount_alice: 100000000n,
  airdrop_amount_bob: 500000000n,
  leaf_alice: [
    228, 20, 163, 158, 201, 12, 147, 212, 38, 41, 236, 245, 197, 123, 230, 202, 119, 180, 19, 47,
    109, 194, 156, 98, 170, 255, 129, 104, 238, 21, 202, 250,
  ],
  leaf_bob: [
    249, 219, 103, 24, 239, 106, 190, 96, 120, 46, 240, 238, 39, 207, 6, 136, 61, 38, 169, 152, 46,
    125, 58, 78, 79, 196, 110, 238, 155, 85, 201, 235,
  ],
  total_supply: 1000000000,
  proof_for_alice: [
    [
      249, 219, 103, 24, 239, 106, 190, 96, 120, 46, 240, 238, 39, 207, 6, 136, 61, 38, 169, 152,
      46, 125, 58, 78, 79, 196, 110, 238, 155, 85, 201, 235,
    ],
  ],
  proof_for_bob: [
    [
      228, 20, 163, 158, 201, 12, 147, 212, 38, 41, 236, 245, 197, 123, 230, 202, 119, 180, 19, 47,
      109, 194, 156, 98, 170, 255, 129, 104, 238, 21, 202, 250,
    ],
  ],
  index_alice: 0,
  index_bob: 1,
  root: [
    26, 91, 204, 71, 229, 32, 84, 69, 114, 107, 220, 158, 119, 253, 74, 52, 228, 187, 194, 83, 224,
    80, 67, 9, 44, 32, 79, 200, 120, 68, 250, 92,
  ],
  creator: "0xe2235a2ffe0354b27a6a1c543be6bf2920ff2134",
}

// --- Example Usage ---
async function main() {
  const leafData: LeafData[] = [
    { recipient: setup.alice_account, value: setup.airdrop_amount_alice },
    { recipient: setup.bob_account, value: setup.airdrop_amount_bob },
  ]

  const merkleTree = new MerkleTree(leafData)

  // Expected zero keccak hash: [173, 50, 40, 182, 118, 247, 211, 205, 66, 132, 165, 68, 63, 23, 241, 150, 43, 54, 228, 145, 179, 10, 64, 178, 64, 88, 73, 229, 151, 186, 95, 181]

  console.log(
    "Merkle Root:",
    Array.from(merkleTree.root!)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join(""),
  )
  console.log(
    "Computed Leaf alice",
    MerkleTree.encodeLeaf(setup.alice_account, setup.airdrop_amount_alice),
    "Expected:",
    setup.leaf_alice,
  )
  console.log(
    "Computed Bob alice",
    MerkleTree.encodeLeaf(setup.bob_account, setup.airdrop_amount_bob),
    "Expected:",
    setup.leaf_bob,
  )
  console.log(
    "Computed hash_0x",
    MerkleTree.hashPair(new Uint8Array().fill(0, 0, 32), new Uint8Array().fill(0, 0, 32)),
    "\nExpected:",
    setup.root,
  )

  // Test generating and verifying a proof for the first leaf
  const leafToProve = MerkleTree.encodeLeaf(leafData[0].recipient, leafData[0].value)
  const proof = merkleTree.getProof(0)

  console.log(
    "\nProof for leaf 0:",
    proof.map((p) =>
      Array.from(p)
        .map((b) => b.toString(16).padStart(2, "0"))
        .join(""),
    ),
  )
  const isValid = MerkleTree.verifyProof(leafToProve, proof, 0, merkleTree.root!)
  console.log("Is leaf 0 valid?", isValid) // Expected: true

  // Test an invalid proof
  const fakeLeaf = MerkleTree.encodeLeaf(setup.creator, 9999n) // Different value
  const isFakeValid = MerkleTree.verifyProof(fakeLeaf, proof, 0, merkleTree.root!)
  console.log("Is fake leaf valid?", isFakeValid) // Expected: false

  // Test a different leaf
  const leafToProve2 = MerkleTree.encodeLeaf(leafData[1].recipient, leafData[1].value)
  const proof2 = merkleTree.getProof(1)
  console.log(
    "\nProof for leaf 2:",
    proof2.map((p) =>
      FixedSizeBinary.fromHex(
        `0x${Array.from(p)
          .map((b) => b.toString(16).padStart(2, "0"))
          .join("")}`,
      ).asBytes(),
    ),
  )
  const isValid2 = MerkleTree.verifyProof(leafToProve2, proof2, 1, merkleTree.root!)
  console.log("Is leaf 2 valid?", isValid2) // Expected: true

  console.log("lapa: ", keccak_256(new Uint8Array().fill(0, 0, 32)))
  console.log(
    "lopo",
    FixedSizeBinary.fromArray([Number(setup.airdrop_amount_alice), 0, 0, 0]).asBytes(),
    Number(setup.airdrop_amount_alice),
  )
}

main().catch(console.error)
