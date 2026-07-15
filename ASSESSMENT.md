# Rust For Bitcoin

## Take-Home Assessment

### Objective

Build a **command-line application in Rust** that communicates with a local **Bitcoin Core** node running on **Regtest** using **Polar**.

The goal of this assessment is to evaluate your Rust programming skills, understanding of Bitcoin and it's Core JSON-RPC, 

---

# Requirements

## Part 1 — Setting Up Bitcoin Core

Use **Polar** to create and run a local Bitcoin Core node on **Regtest**.

Your application should connect to the node using Bitcoin Core's JSON-RPC interface.

Configuration should be provided through one of the following:

* Environment variables
* A configuration file
* Command-line flags

The application should not require editing the source code to change credentials.

Your README should include instructions for:

* Installing Polar
* Creating a Bitcoin Core node
* Starting the network
* Obtaining the RPC URL, username, and password
* Running your application against the node

---

## Part 2 — CLI Commands

Implement the following commands.

### blockchain-info

Display:

* Chain
* Number of blocks
* Number of headers
* Difficulty
* Verification progress

---

### wallet-info

Display:

* Wallet name
* Balance
* Unconfirmed balance
* Number of transactions

---

### balance

Print the wallet balance.

---

### new-address

Generate and print a new receiving address.

---

## Part 3 — Generic RPC Command

Support executing arbitrary Bitcoin Core RPC methods.

Example:

```bash
cargo run -- rpc getblockcount

cargo run -- rpc getblockhash 200

cargo run -- rpc getblock <hash>
```

Arguments should be passed dynamically.

---

# Technical Requirements

* Use **Rust**.
* Build a **command-line application** (no graphical interface).
* Your application should compile successfully using:

```bash
cargo build
```

and run using:

```bash
cargo run
```

Suggested crates (you may use alternatives):

* clap
* serde
* serde_json
* reqwest
* anyhow or thiserror

Where appropriate, deserialize Bitcoin Core RPC responses into strongly typed Rust structs instead of relying solely on generic JSON values.

---

# Error Handling

Your application should gracefully handle:

* Invalid credentials
* Connection failures
* Invalid RPC methods
* Invalid parameters
* Missing wallet

Avoid panics and provide clear, user-friendly error messages.

---

# Project Structure

A clean and modular project structure is expected.

Example:

```
src/
├── main.rs
├── cli.rs
├── rpc.rs
├── config.rs
├── error.rs
└── commands/
    ├── blockchain.rs
    ├── wallet.rs
    └── address.rs
```

---

# Documentation

Include a README that explains:

* Project overview
* Installation
* Setting up Polar
* Running the Bitcoin Core node
* Configuring the application
* Example commands
* Any assumptions or design decisions

---

# Bonus (Optional)

Implement one or more of the following:

* Pretty terminal output
* Configuration file support
* Unit tests
* Async implementation using Tokio
* Logging with `tracing`
* A reusable RPC client abstraction
* Support for multiple wallets

---

# Submission

Submit:

* A GitHub repository
* A README with setup instructions
* Example terminal output demonstrating each implemented command

---

# What We Are Looking For

This assessment is designed to evaluate your ability to:

* Write idiomatic Rust.
* Organize code into reusable modules.
* Interact with Bitcoin Core using JSON-RPC.
* Handle errors gracefully.
* Write maintainable, well-documented code.
* Demonstrate a solid understanding of Rust and Bitcoin developer tooling.
