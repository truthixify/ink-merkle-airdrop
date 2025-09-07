import { getBytes } from 'ethers';
import { keccak_256 } from "@noble/hashes/sha3"
import { FixedSizeBinary } from 'polkadot-api';

// Define a type for your leaf data
export type LeafData = {
    recipient: string; // Ethereum address
    value: bigint;     // Represents a large number (e.g., wei)
};

export class MerkleTree {
  private leaves: Uint8Array[];
  private tree: Uint8Array[][];
  public root: Uint8Array | null;

  constructor(leafData: LeafData[]) {
    this.leaves = leafData.map(data => MerkleTree.encodeLeaf(data.recipient, data.value));
    this.tree = [];
    this.root = null;
    this.buildTree();
  }

  /** keccak256(abi.encodePacked(address, uint256)) */
  public static encodeLeaf(recipient: string, value: bigint): Uint8Array {
    // const encoded = solidityPacked(["address", "uint256"], [recipient, value]);
    // return getBytes(keccak256(encoded)); // one hash only

    return getBytes(keccak_256(
        FixedSizeBinary.fromText("0x" + recipient.replace(/^0x/, '').padStart(64, '0')).asBytes().concat(
            FixedSizeBinary.fromBytes(value).asBytes(),
        )
    ));
  }

  /** keccak256(left || right) */
  public static hashPair(left: Uint8Array, right: Uint8Array): Uint8Array {
    const concatenated = new Uint8Array(left.length + right.length);
    concatenated.set(left, 0);
    concatenated.set(right, left.length);
    return getBytes(keccak_256(concatenated));
  }

  private buildTree(): void {
    if (this.leaves.length === 0) {
      this.root = null;
      return;
    }

    let currentLevel = this.leaves.slice();
    this.tree.push(currentLevel);

    while (currentLevel.length > 1) {
      const nextLevel: Uint8Array[] = [];
      for (let i = 0; i < currentLevel.length; i += 2) {
        const left = currentLevel[i];
        const right = (i + 1 < currentLevel.length) ? currentLevel[i + 1] : left;
        nextLevel.push(MerkleTree.hashPair(left, right));
      }
      this.tree.push(nextLevel);
      currentLevel = nextLevel;
    }

    this.root = currentLevel[0];
  }

  public getProof(index: number): Uint8Array[] {
    if (index < 0 || index >= this.leaves.length) {
      throw new Error("Leaf index out of bounds.");
    }

    const proof: Uint8Array[] = [];
    let currentIndex = index;

    for (let level = 0; level < this.tree.length - 1; level++) {
      const currentLevel = this.tree[level];
      const siblingIndex = currentIndex % 2 === 0 ? currentIndex + 1 : currentIndex - 1;

      if (siblingIndex < currentLevel.length) {
        proof.push(currentLevel[siblingIndex]);
      } else {
        proof.push(currentLevel[currentIndex]); // duplicate if odd
      }
      currentIndex = Math.floor(currentIndex / 2);
    }

    return proof;
  }

  public static verifyProof(leaf: Uint8Array, proof: Uint8Array[], index: number, root: Uint8Array): boolean {
    let computed = leaf;
    let idx = index;

    for (const sibling of proof) {
      if (idx % 2 === 0) {
        computed = MerkleTree.hashPair(computed, sibling);
      } else {
        computed = MerkleTree.hashPair(sibling, computed);
      }
      idx = Math.floor(idx / 2);
    }

    return MerkleTree.bytesEqual(computed, root);
  }

  private static bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (a[i] !== b[i]) return false;
    }
    return true;
  }
}

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

// --- Example Usage ---
async function main() {
    const leafData: LeafData[] = [
        { recipient: setup.alice_account, value: setup.airdrop_amount_alice },
        { recipient: setup.bob_account, value: setup.airdrop_amount_bob },
    ];

    const merkleTree = new MerkleTree(leafData);

    // [173, 50, 40, 182, 118, 247, 211, 205, 66, 132, 165, 68, 63, 23, 241, 150, 43, 54, 228, 145, 179, 10, 64, 178, 64, 88, 73, 229, 151, 186, 95, 181]

    console.log("Merkle Root:", Array.from(merkleTree.root!).map(b => b.toString(16).padStart(2, '0')).join(''));
    console.log("Computed Leaf alice", MerkleTree.encodeLeaf(setup.alice_account, setup.airdrop_amount_alice), "Expected:", setup.leaf_alice);
    console.log("Computed hash_0x", MerkleTree.hashPair(
        new Uint8Array().fill(0, 0, 32),
        new Uint8Array().fill(0, 0, 32),
    ), "\nExpected:", setup.root);

    // Test generating and verifying a proof for the first leaf
    const leafToProve = MerkleTree.encodeLeaf(leafData[0].recipient, leafData[0].value);
    const proof = merkleTree.getProof(0);

    console.log("\nProof for leaf 0:", proof.map(p => Array.from(p).map(b => b.toString(16).padStart(2, '0')).join('')));
    const isValid = MerkleTree.verifyProof(leafToProve, proof, 0, merkleTree.root!);
    console.log("Is leaf 0 valid?", isValid); // Expected: true

    // Test an invalid proof
    const fakeLeaf = MerkleTree.encodeLeaf(setup.creator, 9999n); // Different value
    const isFakeValid = MerkleTree.verifyProof(fakeLeaf, proof, 0, merkleTree.root!);
    console.log("Is fake leaf valid?", isFakeValid); // Expected: false

    // Test a different leaf
    const leafToProve2 = MerkleTree.encodeLeaf(leafData[1].recipient, leafData[1].value);
    const proof2 = merkleTree.getProof(1);
    console.log("\nProof for leaf 2:", proof2.map(p => FixedSizeBinary.fromHex("0x" + Array.from(p).map(b => b.toString(16).padStart(2, '0')).join('')).asBytes()));
    const isValid2 = MerkleTree.verifyProof(leafToProve2, proof2, 1, merkleTree.root!);
    console.log("Is leaf 2 valid?", isValid2); // Expected: true
}

main().catch(console.error);