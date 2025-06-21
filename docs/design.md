### **Design Document: Decentralized Incentive Platform on Nervos**

### 1. High-Level Overview

This document describes a decentralized incentive platform built on the Nervos CKB blockchain. The platform allows **Creators** to fund marketing or engagement campaigns, and **Subscribers** to earn rewards by proving they have consumed specific content.

The system is designed with a "separation of concerns" architecture, leveraging Nervos's Cell Model to ensure security and logical clarity. Core financial logic and rules are enforced by on-chain smart contracts, while off-chain services handle user interaction, data aggregation, and proof verification.

### 2. Core Components

| Component              | Type                | Purpose                                                                                                                            |
| :--------------------- | :------------------ | :--------------------------------------------------------------------------------------------------------------------------------- |
| **Actors**             | -                   | Users who interact with the system.                                                                                                |
| **Smart Contracts**    | On-Chain Logic      | Rust code compiled to RISC-V that enforces the platform's rules, separating authorization (`lock`) from state validation (`type`). |
| **Cell Types**         | On-Chain State      | The "database records" of the blockchain, representing funds and proofs, each protected by specific smart contracts.               |
| **Off-Chain Services** | Centralized Backend | Manages user-facing interactions and processes data before on-chain settlement.                                                    |

### 3. Detailed Component Breakdown

#### A. Actors

1.  **Creator:** The entity that funds a campaign by depositing CKB into a `Vault Cell`. Can also authorize refunds and capacity adjustments.
2.  **Subscriber/Claimant:** The end-user who consumes content, creates a `Proof Cell` to prove it, and later claims a CKB reward.
3.  **Admin:** The platform operator who manages the lifecycle of a campaign. The Admin is responsible for initiating the distribution of rewards and can issue refunds. The Admin's authority is secured by their private key, which is used to sign transactions authorized by the `vault-lock` script.

#### B. Smart Contracts (On-Chain Logic)

The system is composed of five distinct smart contracts, each with a specific responsibility.

1.  **Vault Lock Script (`vault-lock`)**

    - **Purpose:** To authorize actions on the `Vault Cell`.
    - **Key Validations:**
      - Reads the `creator_lock_hash` and `admin_lock_hash` from its script arguments.
      - Verifies that any transaction consuming the `Vault Cell` is co-signed by an input belonging to either the **Admin** (for distribution) or the **Creator** (for refunds/capacity adjustments).

2.  **Vault Type Script (`vault-type`)**

    - **Purpose:** To validate the state transitions of the main `Vault Cell`.
    - **Key Validations:**
      - **Creation:** Validates the initial `VaultCellData`, ensuring the `fee_percentage` is within a valid range (0-10000).
      - **Consumption:** Determines if the action is a "Distribution," "Refund," or "Capacity Adjustment" by examining output cells.
      - **On Distribution:**
        - Verifies that the sum of all output `Distribution Shard Cells` and the `Fee Cell` equals the total `Vault` capacity.
        - Ensures each shard has consistent data (`campaign_id`, `proof_script_code_hash`, etc.) derived from the vault.
        - Ensures exactly one fee cell is created with the correct capacity based on the fee percentage.
      - **On Refund:** Ensures the output is a single cell locked to the `creator_lock_hash` (retrieved from the `vault-lock`'s args).
      - **On Capacity Adjustment:** Ensures the `VaultCellData` remains immutable.

3.  **Proof Type Script (`proof-type`)**

    - **Purpose:** To govern the lifecycle of `Proof Cells`.
    - **Key Validations:**
      - **Creation:**
        - Enforces uniqueness using Type ID.
        - Ensures a `Proof Cell` is created with a valid `ProofCellData` structure.
        - Verifies that the cell's actual lock hash matches the `subscriber_lock_hash` stored in the cell data.
      - **Consumption:** Ensures a `Proof Cell`, once spent, is permanently destroyed and cannot be "updated" or re-created.

4.  **Distribution Lock Script (`distribution-lock`)**

    - **Purpose:** To secure the reward pool in each `Distribution Shard Cell` and authorize individual claims.
    - **Key Validations:**
      - **On Claim:**
        - Computes a leaf hash from the claimant's `Proof Cell` outpoint and subscriber lock hash.
        - Verifies the provided Merkle path against the `merkle_root` stored in the shard's data.
      - **On Reclamation:** Performs no validation. The action is authorized by the absence of a witness, and the time-lock is enforced by the `distribution-type` script.

5.  **Distribution Type Script (`distribution-type`)**
    - **Purpose:** To validate the state transitions of `Distribution Shard Cells`.
    - **Key Validations:**
      - **Creation:** (As part of the vault fan-out) Ensures all created shards have consistent and valid data.
      - **On Claim (Update):**
        - Verifies the transaction structure: exactly one input shard, one output shard, and one reward cell.
        - Ensures the new `Distribution Shard Cell` is an exact clone of the input, with its capacity reduced by exactly the `uniform_reward_amount`.
        - Verifies the `Reward Cell` has the correct capacity and is locked to the subscriber.
        - Validates the integrity of the consumed `Proof Cell` (e.g., matching `campaign_id`).
      - **On Final Claim (Destruction):**
        - Verifies the transaction structure: one input shard, one reward cell, and no new shard.
        - Confirms the input shard's capacity exactly equals the `uniform_reward_amount`.
      - **On Reclamation (Destruction):**
        - Verifies the transaction's `since` field is past the `deadline` stored in the shard's data.
        - Ensures the remaining funds are returned to the `admin_lock_hash`.

#### C. Cell Types (On-Chain State)

In Nervos CKB, each cell has two scripts that serve different purposes:

- **Lock Script:** Controls _who_ can spend (consume) a cell. It's like the "owner's signature" on a check.
- **Type Script:** Enforces rules about _how_ the cell can be created, transformed, or destroyed. It's like the "terms and conditions" of a financial instrument.

1.  **Vault Cell**

    - **Lock Script:** The `vault-lock` script. Its arguments contain the `creator_lock_hash` and `admin_lock_hash`.
      - _Why:_ Only the Creator or Admin can authorize spending this cell.
    - **Type Script:** The `vault-type` script. Its arguments contain the code hashes for the `distribution-lock` and `distribution-type` scripts.
      - _Why:_ Ensures the vault can only be spent in ways that follow campaign rules (proper distribution or refund).
    - **Data:** `VaultCellData` containing:
      - `campaign_id`: Unique identifier for the campaign (32 bytes).
      - `fee_percentage`: Platform fee in basis points (0-10000 for 0-100%).
      - `proof_script_code_hash`: Code hash of the Proof contract (32 bytes).
    - **Purpose:** To hold the entire campaign fund before distribution.

2.  **Proof Cell**

    - **Lock Script:** The Subscriber's standard `secp256k1` lock script.
      - _Why:_ Only the subscriber who created the proof should be able to use it.
    - **Type Script:** The `proof-type` script with a Type ID as args.
      - _Why:_ Ensures the proof is created correctly, can't be duplicated, and can only be consumed once.
    - **Data:** `ProofCellData` containing:
      - `entity_id`: Identifier for the content that was consumed (32 bytes).
      - `campaign_id`: Identifier linking this proof to a specific campaign (32 bytes).
      - `proof`: Cryptographic proof of content consumption (32 bytes).
      - `subscriber_lock_hash`: Lock script hash of the subscriber (32 bytes).
    - **Purpose:** An immutable, on-chain receipt proving a subscriber's interaction.

3.  **Distribution Shard Cell**

    - **Lock Script:** The `distribution-lock` script.
      - _Why:_ This is a key innovation. The lock itself validates Merkle proofs to authorize claims.
    - **Type Script:** The `distribution-type` script.
      - _Why:_ Enforces the accounting and state transition rules for every claim or reclamation action.
    - **Data:** `DistributionCellData` containing:
      - `campaign_id`: Identifier linking this shard to a specific campaign (32 bytes).
      - `admin_lock_hash`: Lock hash of the admin for reclaiming funds (32 bytes).
      - `merkle_root`: Root of the Merkle tree for authorized claimants in this shard (32 bytes).
      - `proof_script_code_hash`: Code hash of the Proof contract (32 bytes).
      - `uniform_reward_amount`: Amount of CKB each claimant receives (8 bytes).
      - `deadline`: Timestamp after which funds can be reclaimed by the admin (8 bytes).
    - **Purpose:** To hold a fraction of the total reward pool, allowing for concurrent claims.

4.  **Fee Cell & Reward Cell**
    - **Lock Script:** Standard `secp256k1` lock (Admin's for Fee, Subscriber's for Reward).
      - _Why:_ These are simple payment cells that transfer value to their respective owners.
    - **Type Script:** `null`.
      - _Why:_ No additional rules are needed beyond ownership.
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
    - **Lock:** `vault-lock` script (with Creator and Admin lock hashes as args).
    - **Type:** `vault-type` script.
    - **Data:** Campaign ID, fee percentage (e.g., 5%).
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
2.  The tooling fetches the list of all verified claimants from the Backend.
3.  The Admin decides to create a number of shards (e.g., 10). The tooling partitions the claimants into 10 groups.
4.  For each group, it generates a unique Merkle root and calculates the required CKB.
5.  The tooling constructs a single large transaction that:
    - **Consumes:** The 10,000 CKB `Vault Cell` (authorized by the `vault-lock` script via the Admin's signature).
    - **Creates:**
      - 10 `Distribution Shard Cells`, each with its own capacity and Merkle root.
      - 1 `Fee Cell` with 500 CKB (5% of 10,000), locked to the Admin.
6.  The `vault-type` script runs and validates this entire state transition. The Admin signs and sends the transaction.

**Phase 5: Reward Claim (Subscriber Action)**

1.  A **Subscriber** wants to claim their reward. Their client calls the **Backend API**.
2.  The Backend provides the subscriber with their personal Merkle proof, the outpoint of their assigned `Distribution Shard Cell`, and their `Proof Cell` outpoint.
3.  The subscriber's client constructs a claim transaction that:
    - **Consumes:** Their own `Proof Cell` and the `Distribution Shard Cell`.
    - **Creates:**
      - A `Reward Cell` of 9.5 CKB, locked to the subscriber.
      - A new `Distribution Shard Cell` with reduced capacity.
    - **Includes:** A witness containing the Merkle proof.
4.  The transaction is sent. The `distribution-lock` script runs, verifying the Merkle proof. The `distribution-type` script runs, verifying the state change (capacities, new shard creation, etc.). This design allows many subscribers to claim their rewards in parallel, each interacting with a different shard.
5.  For the final claim in a shard, no new shard cell is created, just the reward cell.

**Phase 6: Campaign Refund (Optional)**

1.  If a campaign is canceled, the **Admin** or **Creator** constructs a transaction.
2.  It **consumes** the `Vault Cell`. The `vault-lock` script verifies the transaction is signed by an authorized party.
3.  It **creates** a single new cell with the funds, locked to the **Creator's** address.
4.  The `vault-type` script runs, sees it's a refund action, and validates it.
