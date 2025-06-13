## **Polymeer Litepaper**

**Verifiable Incentives, Scaled for Growth**

**Version 1.0**

### **Abstract**

Polymeer is a decentralized incentive and user engagement protocol built on the Nervos CKB blockchain. It introduces a novel framework for creators to reward their audience for verifiable actions, all while prioritizing user privacy and a seamless Web2-like experience. Our ecosystem consists of a user-friendly web portal for campaign management, a privacy-centric browser extension for tracking engagement, and a suite of highly-audited smart contracts. By leveraging Nervos's unique Cell Model and integrating with passwordless wallets like JoyID, Polymeer delivers a fully anonymous, transparent, and scalable solution for the next generation of the creator economy.

---

### **1. The Vision: Rebuilding Trust in the Digital Attention Economy**

The digital relationship between creators and their audience is broken. Users' attention is monetized by opaque, centralized platforms, while creators struggle with inefficient, high-fee tools to genuinely reward their most engaged followers. Existing Web3 solutions, while promising decentralization, often fail on user experience, demanding complex wallet interactions and suffering from prohibitive transaction costs.

Polymeer is designed from the ground up to fix this. We envision a future where:

- **Engagement is Verifiable and Directly Rewarded:** Users are compensated for their attention in a transparent, on-chain manner.
- **Privacy is Non-Negotiable:** User participation is fully anonymous, free from the data harvesting of traditional platforms.
- **The Experience is Seamless:** Interacting with a decentralized protocol feels as simple as using a modern web application.

### **2. The Polymeer Ecosystem: A Three-Pillar Approach**

Our solution is delivered through three core components designed to work in harmony.

#### **A. The Polymeer Web Extension**

The gateway for subscribers. This lightweight browser extension is the user's private engagement tracker.

- **Session Tracking:** It passively and privately monitors interactions with partnered creator content (e.g., time spent on a page, video watch percentage, clicks). This data never leaves the user's device.
- **Proof Generation:** When an engagement milestone is met, the extension helps the user generate a cryptographic "proof of action" and create their on-chain `Proof Cell` with a single click.
- **Effortless Onboarding:** Integrates directly with passwordless, passkey-based wallets like **JoyID**, allowing users to create a non-custodial wallet with just their fingerprint or Face ID. No seed phrases, no complex setup.

#### **B. The Polymeer Web Portal**

The command center for creators and the rewards hub for subscribers.

- **For Creators:** A no-code dashboard to:
  - Launch and fund reward campaigns by creating on-chain `Vault Cells`.
  - Define engagement rules (e.g., "watch 90% of a video," "visit three pages").
  - Monitor campaign statistics and claimant counts in real-time.
- **For Subscribers:** A unified interface to:
  - Discover active campaigns from their favorite creators.
  - Track the status of their submitted `Proof Cells`.
  - Claim their CKB rewards with a simple, one-click process.

#### **C. The Polymeer Protocol (On-Chain Contracts)**

The immutable, decentralized backbone that guarantees the system's integrity.

- **Vault Contract (Type Script):** Secures campaign funds and enforces the rules of distribution and refunds. It is controlled by the Polymeer Admin but its rules are transparent and unchangeable.
- **Proof Contract (Type Script):** Governs the creation and single-use nature of a subscriber's on-chain proof, ensuring it cannot be double-spent.
- **Distribution Contract (Lock Script):** A highly-efficient contract that secures sharded reward pools and processes thousands of concurrent claims by verifying Merkle proofs.

### **3. How It Works: The Flow of Value**

1.  **Onboarding (Anonymous):** A user installs the **Polymeer Extension**. When prompted, they use their device's biometrics (fingerprint/Face ID) to instantly create a **JoyID wallet**. Their on-chain identity is now secure and totally anonymous.

2.  **Campaign Launch:** A creator uses the **Polymeer Portal** to deposit 10,000 CKB into a campaign `Vault Cell`. They set the rules and a 5% platform fee.

3.  **Private Engagement Tracking:** The user browses the creator's content. The **Polymeer Extension** privately notes that they have watched the required 90% of a video.

4.  **On-Chain Proof:** The extension prompts the user: "You've earned a proof from this creator. Create it now?" With one click and a biometric scan, the user signs a transaction to create their unique `Proof Cell` on the Nervos blockchain.

5.  **Off-Chain Verification:** The extension sends the `Proof Cell`'s on-chain location to Polymeer's backend service for inclusion in the claimant list. This step is purely for aggregation; the _validity_ of the proof is already guaranteed by the on-chain cell.

6.  **Distribution Fan-Out:** The campaign ends. The Polymeer Admin uses their tooling to consume the `Vault Cell` and create multiple `Distribution Shard Cells`, each containing a reward pool and a Merkle root of verified claimants.

7.  **One-Click Claim:** The user visits the **Polymeer Portal**. It shows "You have 1 claimable reward." The portal fetches the user's Merkle proof from the backend. The user clicks "Claim," authenticates with their biometrics via JoyID, and the protocol executes the claim, sending CKB directly to their wallet.

### **4. Technical Deep Dive: Why This Architecture Excels**

- **Privacy by Design:** JoyID's passkey-based system and our local-first tracking mean Polymeer never has access to user emails, passwords, or personal data. All on-chain activity is pseudonymous.
- **True Scalability:** The "fan-out" from a single `Vault Cell` to many `Distribution Shard Cells` is the key innovation. It parallelizes the claim process, solving the state contention problem that plagues other UTXO-based systems.
- **Separation of Concerns:**
  - **Authentication:** Handled by JoyID and the user's device biometrics.
  - **Authorization & Rules:** Enforced by our immutable on-chain smart contracts.
  - **Usability & Data Aggregation:** Managed by our off-chain web portal and extension.
    This separation makes the system both highly secure and user-friendly.

### **5. Business Model & Tokenomics**

The protocol's business model is simple, transparent, and encoded on-chain. The `VaultCellData` contains a `fee_percentage` field. During the "Fan-Out," the `Vault` contract enforces that this percentage of the total campaign fund is sent to a `Fee Cell` controlled by the Admin. All CKB amounts are handled directly, with no separate protocol token required.

### **6. Why Nervos CKB?**

Polymeer is uniquely suited to Nervos CKB and would be significantly more complex and less secure on other platforms.

- **Cell Model:** Allows us to represent every logical component (vaults, proofs, shards) as a distinct, ownable on-chain object.
- **Low-Level VM (RISC-V):** Enables us to write highly-optimized, secure contracts in a modern language like Rust.
- **UTXO for Concurrency:** The ability to spend multiple inputs and create multiple outputs in one transaction is what makes the sharding model and the "Fan-Out" possible, enabling massive parallelism for claims.
- **First-Class State:** CKB as a measure of state storage means that all on-chain data pays for its own existence, creating a sustainable and predictable economic model.

### **7. Use Cases & Future Vision**

Polymeer is a foundational protocol for any "Proof-of-Action" scenario:

- **Ad-Tech:** Reward users for watching ads or engaging with content.
- **Learn-to-Earn:** Reward students for completing courses and passing quizzes.
- **Community Engagement:** Airdrop tokens to users who complete specific community tasks (e.g., social media follows, governance votes).
- **Decentralized User Testing:** Reward users for finding bugs or providing feedback.

### **8. Conclusion**

Polymeer is not just another rewards platform; it is a fundamental rethinking of the value exchange between creators and their communities. By launching with a polished suite of tools including a privacy-first web extension and a seamless portal powered by JoyID, we will offer a solution that is immediately accessible, scalable, and built on a verifiable foundation of trust. We are building the rails for a more equitable and transparent creator economy.
