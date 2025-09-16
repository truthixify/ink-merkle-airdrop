import { keccak_256 } from "@noble/hashes/sha3"
import { getBytes, toBeHex } from "ethers"

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
        nextLevel.push(MerkleTree.hashPair(left as Uint8Array, right as Uint8Array))
      }
      this.tree.push(nextLevel)
      currentLevel = nextLevel
    }

    this.root = currentLevel[0] as Uint8Array
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
      const currentLevel = this.tree[level] as Uint8Array[]
      const siblingIndex = currentIndex % 2 === 0 ? currentIndex + 1 : currentIndex - 1

      if (siblingIndex < currentLevel.length) {
        proof.push(currentLevel[siblingIndex] as Uint8Array)
      } else {
        proof.push(currentLevel[currentIndex] as Uint8Array) // duplicate if odd
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
