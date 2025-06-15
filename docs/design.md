### **Design Document: Decentralized Incentive Platform on Nervos**

### 1. High-Level Overview

This document describes a decentralized incentive platform built on the Nervos CKB blockchain. The platform allows **Creators** to fund marketing or engagement campaigns, and **Subscribers** to earn rewards by proving they have consumed specific content.

The system is designed with a "separation of concerns" architecture, leveraging Nervos's Cell Model to ensure security and logical clarity. Core financial logic and rules are enforced by on-chain smart contracts, while off-chain services handle user interaction, data aggregation, and proof verification.

### 2. Core Components

| Component              | Type                | Purpose                                                                         |
| :--------------------- | :------------------ | :------------------------------------------------------------------------------ |
| **Actors**             | -                   | Users who interact with the system.                                             |
| **Smart Contracts**    | On-Chain Logic      | Rust code compiled to RISC-V that enforces the platform's rules.                |
| **Cell Types**         | On-Chain State      | The "database records" of the blockchain, representing funds and proofs.        |
| **Off-Chain Services** | Centralized Backend | Manages user-facing interactions and processes data before on-chain settlement. |

### 3. Detailed Component Breakdown

#### A. Actors

1.  **Creator:** The entity that funds a campaign by depositing CKB into a `Vault Cell`.
2.  **Subscriber/Claimant:** The end-user who consumes content, creates a `Proof Cell` to prove it, and later claims a CKB reward.
3.  **Admin:** The platform operator who manages the lifecycle of a campaign. The Admin is responsible for initiating the distribution of rewards and can issue refunds. The Admin's authority is secured by their private key.

#### B. Smart Contracts (On-Chain Logic)

1.  **Proof Contract (Type Script)**

    - **Purpose:** To govern the lifecycle of `Proof Cells`.
    - **Key Validations:**
      - **Creation:** 
        - Ensures a `Proof Cell` is created with a valid data structure (`ProofCellData`).
        - Validates that critical fields like `entity_id`, `campaign_id`, and `proof` are not empty/null.
        - Verifies that the cell's actual lock hash matches the `subscriber_lock_hash` stored in the cell data.
      - **Uniqueness:** Enforces that only one `Proof Cell` of this type is created per transaction using Type ID validation.
      - **Consumption:** Ensures a `Proof Cell`, once spent, is permanently destroyed and cannot be "updated" or re-created.
      - **Prevention:** Explicitly blocks any attempt to "update" a `Proof Cell` by creating a new one with the same Type ID.

2.  **Vault Contract (Type Script)**

    - **Purpose:** To validate the state transitions of the main `Vault Cell`.
    - **Key Validations:**
      - **Creation:** 
        - Validates the initial `VaultCellData`, ensuring the `fee_percentage` is within a valid range (0-100%).
        - Checks that critical fields like `campaign_id`, `creator_lock_hash`, and `proof_script_code_hash` are not null.
        - Verifies the script arguments are correctly formatted (must be a 32-byte hash).
      - **Consumption:** Determines if the action is a "Distribution" or a "Refund" by examining output cells.
      - **On Distribution:**
        - Verifies that the sum of all output `Distribution Shard Cells` and the `Fee Cell` equals the total `Vault` capacity.
        - Ensures each shard has a unique, sequential `shard_id` starting from 0.
        - Verifies that the data within each shard (e.g., `campaign_id`, `proof_script_code_hash`) is consistent with the `Vault`.
        - **Crucially, verifies that the `uniform_reward_amount` is identical across all created shards and makes mathematical sense for the total capacity.**
        - Ensures exactly one fee cell is created with the correct capacity based on the fee percentage.
      - **On Refund:** Ensures the output is a single cell locked to the `creator_lock_hash` stored in the vault data.

3.  **Distribution Contract (Lock Script)**
    - **Purpose:** To secure the reward pool in each `Distribution Shard Cell` and process individual claims.
    - **Key Validations:**
      - **Merkle Proof:** 
        - Computes a leaf hash from the claimant's `Proof Cell` outpoint and subscriber lock hash.
        - Verifies the Merkle path by walking up the tree using the provided sibling hashes.
        - Confirms the computed root matches the `merkle_root` stored in the shard's data.
      - **Proof Cell Integrity:**
        - Finds the input `Proof Cell` by its `type script`'s code hash.
        - Verifies the `Proof Cell`'s outpoint matches the one in the claim witness.
        - Ensures the `Proof Cell`'s campaign ID matches the shard's campaign ID.
        - Confirms the `Proof Cell`'s subscriber lock hash matches the one in the claim witness.
        - Verifies the `Proof Cell`'s lock hash matches the subscriber's lock hash.
      - **Transaction Structure:** 
        - For normal claims: Enforces exactly one `Reward Cell` and one new `Distribution Shard Cell`.
        - For final claims (when the shard is depleted): Allows only one `Reward Cell` with no new shard.
      - **State Transition:** 
        - Ensures the new `Distribution Shard Cell` is an exact clone of the input, with identical type script and data.
        - Verifies the new shard's capacity is reduced by exactly the reward amount.
        - For the reward cell, confirms it has exactly the uniform reward amount and is locked to the subscriber.

#### C. Cell Types (On-Chain State)

In Nervos CKB, each cell has two scripts that serve different purposes:
- **Lock Script:** Controls who can spend (consume) a cell. It's like the "owner's signature" on a check.
- **Type Script:** Enforces rules about how the cell can be created, transformed, or destroyed. It's like the "terms and conditions" of a financial instrument.

1.  **Vault Cell**

    - **Lock Script:** The Admin's standard `secp256k1` lock script.
      - *Why:* Only the Admin should be able to initiate distribution or refund actions.
    - **Type Script:** The `Vault` contract with the Distribution contract's code hash as args.
      - *Why:* Ensures the vault can only be spent in ways that follow campaign rules (proper distribution or refund).
    - **Data:** `VaultCellData` containing:
      - `campaign_id`: Unique identifier for the campaign (32 bytes).
      - `creator_lock_hash`: Lock script hash of the creator for potential refunds (32 bytes).
      - `proof_script_code_hash`: Code hash of the Proof contract (32 bytes).
      - `fee_percentage`: Platform fee percentage in basis points (0-10000 for 0-100%).
    - **Purpose:** To hold the entire campaign fund before distribution.

2.  **Proof Cell**

    - **Lock Script:** The Subscriber's standard `secp256k1` lock script.
      - *Why:* Only the subscriber who created the proof should be able to use it to claim a reward.
    - **Type Script:** The `Proof` contract with a Type ID as args.
      - *Why:* Ensures the proof is created correctly, can't be duplicated, and can only be consumed once.
    - **Data:** `ProofCellData` containing:
      - `entity_id`: Identifier for the content that was consumed (32 bytes).
      - `campaign_id`: Identifier linking this proof to a specific campaign (32 bytes).
      - `proof`: Cryptographic proof of content consumption (32 bytes).
      - `subscriber_lock_hash`: Lock script hash of the subscriber (32 bytes).
    - **Purpose:** An immutable, on-chain receipt proving a subscriber's interaction.

3.  **Distribution Shard Cell**

    - **Lock Script:** The `Distribution` contract.
      - *Why:* This is the key innovation - instead of using a standard lock, we use a custom contract that validates Merkle proofs and enforces claim rules.
    - **Type Script:** `null`.
      - *Why:* No type script is needed because the lock script already enforces all the necessary rules for this cell.
    - **Data:** `DistributionCellData` containing:
      - `campaign_id`: Identifier linking this shard to a specific campaign (32 bytes).
      - `proof_script_code_hash`: Code hash of the Proof contract (32 bytes).
      - `merkle_root`: Root of the Merkle tree for authorized claimants in this shard (32 bytes).
      - `uniform_reward_amount`: Amount of CKB each claimant receives (8 bytes).
      - `shard_id`: Sequential identifier for this shard (4 bytes).
    - **Purpose:** To hold a fraction of the total reward pool, allowing for concurrent claims.

4.  **Fee Cell & Reward Cell**
    - **Lock Script:** Standard `secp256k1` lock (Admin's for Fee, Subscriber's for Reward).
      - *Why:* These are simple payment cells that transfer value to their respective owners.
    - **Type Script:** `null`.
      - *Why:* No additional rules are needed beyond ownership; these are standard value transfer cells.
    - **Data:** `empty`.
    - **Purpose:** Standard CKB cells for transferring value.

#### D. Off-Chain Services

1.  **Backend API & Indexer**

    - **Responsibilities:**
      - Receives `Proof Cell` data from subscribers.
      - Performs off-chain verification of the "proof" (e.g., was the video watched? was the quiz answer correct?).
      - Maintains a database of verified claimants for each campaign.
      - Provides claimant subsets to the Admin for Merkle tree generation.
      - Provides individual Merkle proofs and current `Distribution Shard Cell` outpoints to subscribers during the claim phase.
      - **This is a centralized point of trust for proof verification and data serving.**

2.  **Admin Tooling (Client-Side)**
    - **Responsibilities:**
      - A UI or CLI tool for the Admin.
      - Constructs the "Fan-Out" transaction that consumes the `Vault Cell` and creates the `Distribution Shard Cells`.
      - Handles the partitioning of claimants, generation of Merkle trees for each shard, and calculation of shard capacities.
      - Constructs refund transactions.

### 4. End-to-End Business Flow

**Phase 1: Campaign Creation**

1.  A **Creator** decides to start a campaign and deposits, for example, 10,000 CKB.
2.  The platform (via Admin Tooling) helps the Creator construct a transaction.
3.  This transaction creates a single **`Vault Cell`** on-chain.
    - **Lock:** Admin's address.
    - **Type:** `Vault` contract.
    - **Data:** Campaign ID, Creator's refund address, fee percentage (e.g., 5%).
    - **Capacity:** 10,000 CKB.

**Phase 2: Content Consumption & Proof**

1.  A **Subscriber** consumes a piece of content associated with the campaign.
2.  Their client/wallet creates a transaction to generate a **`Proof Cell`** on-chain. This costs the subscriber a small amount of CKB for the cell's capacity.
3.  The Subscriber's client then sends the `Proof Cell`'s outpoint and proof data to the **Backend API**.

**Phase 3: Off-Chain Verification**

1.  The **Backend** receives the submission.
2.  It verifies the off-chain proof (e.g., checks its database, validates a hash).
3.  If valid, the Backend adds the subscriber's `Proof Cell` outpoint and lock hash to a list of "verified claimants" for that campaign.

**Phase 4: Distribution Fan-Out (Admin Action)**

1.  The campaign ends. The **Admin** uses their **Admin Tooling**.
2.  The tooling fetches the list of all 1,000 verified claimants from the Backend.
3.  The Admin decides to create 10 shards. The tooling partitions the 1,000 claimants into 10 groups of 100.
4.  For each group, it generates a unique Merkle root and calculates the required CKB (e.g., `100 claimants * 9.5 CKB/claimant = 950 CKB`).
5.  The tooling constructs a single large transaction that:
    - **Consumes:** The 10,000 CKB `Vault Cell`.
    - **Creates:**
      - 10 `Distribution Shard Cells`, each with 950 CKB capacity and its own Merkle root.
      - 1 `Fee Cell` with 500 CKB (5% of 10,000), locked to the Admin.
6.  The Admin signs this transaction with their key, and it's sent to the network.

**Phase 5: Reward Claim (Subscriber Action)**

1.  A **Subscriber** wants to claim their reward. Their client calls the **Backend API**.
2.  The Backend provides the subscriber with:
    - Their personal Merkle proof (a path of sibling hashes from their leaf to the root)
    - The current outpoint of their assigned `Distribution Shard Cell`
    - The outpoint of their own `Proof Cell`
3.  The subscriber's client constructs a claim transaction that:
    - **Consumes:** Their own `Proof Cell` and the `Distribution Shard Cell`.
    - **Creates:**
      - A `Reward Cell` of 9.5 CKB, locked to the subscriber.
      - A new `Distribution Shard Cell` with reduced capacity (e.g., `950 - 9.5 = 940.5 CKB`).
    - **Includes:** A witness containing the Merkle proof, the `Proof Cell` outpoint, and the subscriber's lock hash.
4.  The transaction is sent. The `Distribution` lock script runs, verifies everything, and the claim succeeds. Multiple subscribers can do this in parallel on different shards.
5.  For the final claim in a shard, no new shard cell is created, just the reward cell.

**Phase 6: Campaign Refund (Optional)**

1.  If a campaign is canceled, the **Admin** constructs a simple transaction.
2.  It **consumes** the `Vault Cell`.
3.  It **creates** a single new cell with the funds, locked to the **Creator's** address stored in the vault data.
4.  Our `Vault` Type Script runs, sees it's a refund action, and validates it.
