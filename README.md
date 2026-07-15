# rfb — a Bitcoin Core JSON-RPC command-line client

A small Rust CLI that talks to a Bitcoin Core node over its JSON-RPC interface,
built against a regtest node run by [Polar](https://lightningpolar.com/).

It offers four typed commands for the things you check constantly
(`blockchain-info`, `wallet-info`, `balance`, `new-address`) and a generic `rpc`
passthrough for everything else.

> The original assessment brief is preserved in [ASSESSMENT.md](ASSESSMENT.md).

---

## Contents

- [Requirements](#requirements)
- [Installation](#installation)
- [Setting up Polar](#setting-up-polar)
- [Configuration](#configuration)
- [Commands](#commands)
- [Error handling](#error-handling)
- [Multiple wallets](#multiple-wallets)
- [Project structure](#project-structure)
- [Tests](#tests)
- [Design decisions and assumptions](#design-decisions-and-assumptions)

---

## Requirements

- **Rust** 1.85 or newer (this uses the 2024 edition). Install via [rustup](https://rustup.rs/).
- **Docker Desktop** — Polar runs each node in a container.
- **Polar** — see below.

---

## Installation

```bash
git clone <your-repo-url>
cd rfb-assessment
cargo build
```

The binary lands at `target/debug/rfb`. Every example below can be run either as
`cargo run -- <command>` or directly as `./target/debug/rfb <command>`.

---

## Setting up Polar

### 1. Install Docker Desktop

Polar drives Docker, so Docker must be installed and **running** first.
Download it from [docker.com](https://www.docker.com/products/docker-desktop/)
and launch it.

### 2. Install Polar

Download the build for your platform from the
[Polar releases page](https://github.com/jamaljsr/polar/releases) (or from
[lightningpolar.com](https://lightningpolar.com/)), then install it as you would
any other app.

### 3. Create a Bitcoin Core node

1. Open Polar and click **Create Network**.
2. Give it a name (for example `test`).
3. Drag a **Bitcoin Core** node onto the canvas — or set the Lightning node
   counts to zero and the Bitcoin count to one, since this project only needs
   the Bitcoin node.
4. Click **Create Network**.

### 4. Start the network

Press **Start** on the network. Docker pulls the images on first run, so this
takes a minute or two. Wait until the node's status dot turns green.

Polar mines 100+ blocks into the node's default wallet automatically, so the
wallet has a spendable balance immediately. (Bitcoin's 100-block coinbase
maturity rule means most of it starts out *immature* — see
[`wallet-info`](#wallet-info).)

### 5. Find the RPC URL, username and password

Click the Bitcoin node, then open the **Connect** tab. It lists the node's RPC
details. On a stock Polar install these are:

| Setting  | Value                   |
| -------- | ----------------------- |
| RPC URL  | `http://127.0.0.1:18443` |
| Username | `polaruser`             |
| Password | `polarpass`             |

The RPC port is per-node, so a second Bitcoin node gets a different one — always
read the real values off the **Connect** tab.

**These exact values are this CLI's built-in defaults**, so against a stock
Polar node it works with no configuration at all:

```bash
cargo run -- blockchain-info
```

---

## Configuration

Settings resolve from four sources. Highest priority first:

| Priority | Source                | Example                                    |
| -------- | --------------------- | ------------------------------------------ |
| 1        | Command-line flag     | `--rpc-password polarpass`                 |
| 2        | Environment variable  | `BITCOIN_RPC_PASSWORD=polarpass`           |
| 3        | Config file           | `rpc_password = "polarpass"` in `config.toml` |
| 4        | Built-in default      | Polar's defaults, shown above              |

Credentials never need to be edited into the source.

### Flags

| Flag                     | Environment variable    | Default                  |
| ------------------------ | ----------------------- | ------------------------ |
| `--rpc-url <URL>`        | `BITCOIN_RPC_URL`       | `http://127.0.0.1:18443` |
| `--rpc-user <USER>`      | `BITCOIN_RPC_USER`      | `polaruser`              |
| `--rpc-password <PASS>`  | `BITCOIN_RPC_PASSWORD`  | `polarpass`              |
| `--wallet <NAME>`        | `BITCOIN_RPC_WALLET`    | the node's default wallet |
| `--config <PATH>`        | `RFB_CONFIG`            | `./config.toml` if present |
| `-v, --verbose`          | —                       | off                      |

### Config file

Copy the example and edit it:

```bash
cp config.example.toml config.toml
```

`config.toml` is loaded automatically when present and is **git-ignored**, so
local credentials stay out of the repository. Point somewhere else with
`--config <path>`. A missing `config.toml` is fine; a missing file named
explicitly via `--config` is an error.

```toml
rpc_url = "http://127.0.0.1:18443"
rpc_user = "polaruser"
rpc_password = "polarpass"
# wallet = "alice"
```

### Verbose logging

`-v` logs each RPC request and response to stderr, so normal output stays
pipeable. `RUST_LOG` overrides it entirely (`RUST_LOG=rfb=debug`).

```console
$ cargo run -- -v rpc getblockcount
DEBUG rpc request url=http://127.0.0.1:18443/ method="getblockcount" params=[]
DEBUG rpc response status=200 OK body={"result":101,"error":null,"id":"rfb"}
101
```

---

## Commands

All output below is real, captured against a stock Polar regtest node at height 101.

### `blockchain-info`

```console
$ cargo run -- blockchain-info

Blockchain
  Chain:                 regtest
  Blocks:                101
  Headers:               101
  Difficulty:            4.6565423739069247e-10
  Verification progress: 79.02%
```

### `wallet-info`

```console
$ cargo run -- wallet-info

Wallet
  Name:                  <default>
  Balance:               50.00000000 BTC
  Unconfirmed balance:   0.00000000 BTC
  Transactions:          101
  Immature balance:      5000.00000000 BTC
```

Polar's default wallet is named with the empty string, which is shown as
`<default>` rather than as blank space.

The **immature balance** is the coinbase reward from blocks that have not yet
reached the 100-confirmation maturity rule. It is not required by the brief, but
on a fresh regtest node it holds most of the funds, and showing only the
spendable 50 BTC looks like a bug. It is hidden when zero.

### `balance`

```console
$ cargo run -- balance
50.00000000 BTC
```

### `new-address`

```console
$ cargo run -- new-address
bcrt1qx4lnwmluh9n4c8lnrw9pp4dq3gndqpuwvmxyg6
```

Printed bare so it can be piped. An optional `--label` files the address under a
label in the wallet:

```console
$ cargo run -- new-address --label savings
bcrt1qwgmntzmg28lk3ry0kgn34xpu8fyyc2lapaemxp
```

### `rpc` — the generic passthrough

Calls any method the node exposes.

```console
$ cargo run -- rpc getblockcount
101
```

Arguments are typed correctly rather than sent blindly as strings (see
[Design decisions](#argument-types-in-the-generic-rpc-command)):

```console
$ cargo run -- rpc getblockhash 100
6605b9110ebddbb34dcca024bdd84ef79ef37517792a48d6e43322b4257e8769
```

Object and array results are pretty-printed; bare strings are printed unquoted,
so commands compose:

```console
$ cargo run -- rpc getblock $(cargo run -q -- rpc getblockhash 100)
{
  "bits": "207fffff",
  "chainwork": "00000000000000000000000000000000000000000000000000000000000000ca",
  "confirmations": 2,
  "difficulty": 4.6565423739069247e-10,
  "hash": "6605b9110ebddbb34dcca024bdd84ef79ef37517792a48d6e43322b4257e8769",
  "height": 100,
  "mediantime": 1784105025,
  "merkleroot": "11cc0f2e51e96e3d0dce3c15bbd9a6f9665ade96dda21597cf3cf85dee809637",
  "nTx": 1,
  "nextblockhash": "7ad1d1efddaa8612f46c0e66473a371aaf1c1ef61b8317609a5d3081d912bca1",
  "nonce": 0,
  ...
}
```

Because every argument after the method name is forwarded to the node,
**connection flags must come before the subcommand**:

```bash
cargo run -- --rpc-url http://127.0.0.1:18443 rpc getblockcount
```

---

## Error handling

Every failure exits non-zero with a message on stderr. Nothing panics.

**Invalid credentials**

```console
$ cargo run -- --rpc-password wrongpass balance
error: authentication failed for user 'polaruser'
hint: check the RPC username and password. Polar's defaults are polaruser / polarpass (Polar shows them on the node's "Connect" tab).
```

**Connection failure**

```console
$ cargo run -- --rpc-url http://127.0.0.1:19999 blockchain-info
error: could not reach Bitcoin Core at http://127.0.0.1:19999/
hint: is your Polar network running? Check the RPC URL, or start the network in Polar.
  caused by: error sending request for url (http://127.0.0.1:19999/)
  caused by: client error (Connect)
  caused by: tcp connect error
  caused by: Connection refused (os error 61)
```

**Invalid RPC method**

```console
$ cargo run -- rpc notarealmethod
error: 'notarealmethod' is not a known Bitcoin Core RPC method
hint: run `rfb rpc help` to list every method this node supports.
```

**Invalid parameters**

```console
$ cargo run -- rpc getblockhash 200
error: invalid parameters for 'getblockhash': Block height out of range
```

**Missing wallet**

```console
$ cargo run -- --wallet ghostwallet balance
error: wallet 'ghostwallet' is not loaded
hint: create or load a wallet in Polar, or select one with --wallet <name>. Run `rfb rpc listwallets` to see what is currently loaded.
```

**Ambiguous wallet** — when several are loaded and none was named:

```console
$ cargo run -- rpc getbalance
error: this node has more than one wallet loaded, so 'getbalance' needs to know which one to use
hint: pass --wallet <name>. Run `rfb rpc listwallets` to see what is loaded.
```

**Malformed configuration** is caught before any request goes out:

```console
$ cargo run -- --rpc-url 'not a url' balance
error: invalid RPC URL 'not a url': relative URL without a base

$ cargo run -- --config /tmp/nope.toml balance
error: could not read config file '/tmp/nope.toml'
  caused by: No such file or directory (os error 2)
```

---

## Multiple wallets

`--wallet <name>` selects the wallet for `wallet-info`, `balance` and
`new-address`. A wallet can be created through the passthrough:

```console
$ cargo run -- rpc createwallet alice
{
  "name": "alice"
}

$ cargo run -- rpc listwallets
[
  "",
  "alice"
]

$ cargo run -- --wallet alice wallet-info

Wallet
  Name:                  alice
  Balance:               0.00000000 BTC
  Unconfirmed balance:   0.00000000 BTC
  Transactions:          0
```

---

## Project structure

```
src/
├── main.rs               — wiring: parse, dispatch, report errors, set exit code
├── cli.rs                — clap definitions
├── config.rs             — layering flags / env / file / defaults
├── error.rs              — error enum and RPC-code mapping
├── rpc.rs                — the JSON-RPC client
└── commands/
    ├── mod.rs            — shared output formatting
    ├── blockchain.rs     — blockchain-info
    ├── wallet.rs         — wallet-info, balance
    ├── address.rs        — new-address
    └── generic.rs        — the rpc passthrough
```

This follows the layout suggested in the brief. `commands/generic.rs` is the one
addition — the brief's example tree has no home for the Part 3 passthrough.

---

## Tests

```bash
cargo test
```

42 tests, and no running node required — the transport tests stub the node with
[`wiremock`](https://docs.rs/wiremock/).

**Pure logic**

- **`commands/generic.rs`** — argument coercion, including the `getblockhash 200`
  case that motivates it.
- **`error.rs`** — every RPC error code maps to the intended variant.
- **`config.rs`** — the precedence chain, and whether a missing config file is an
  error.
- **`rpc.rs`** — wallet endpoint URL construction, including percent-encoding and
  the empty-named default wallet.
- **`commands/mod.rs`** — output formatting.

**Transport** (`rpc.rs`, against a stub server) — the combinations that are
awkward to provoke from a live node on demand:

- a JSON-RPC 1.0 envelope goes out, with credentials as HTTP basic auth;
- `call` deserializes both a bare value and a struct through the same path;
- 401 with an *empty* body maps to an auth error;
- 404 and 500 responses are still read for their JSON error bodies;
- a non-JSON body maps to an HTTP error rather than a parse failure;
- a result of the wrong shape maps to a decode error, distinct from a node-side
  error;
- wallet calls route to `/wallet/<name>` and others to `/`;
- a refused connection maps to a connection error.

The four tests covering the 404/500 body handling were checked by injecting the
bug they guard against — an early `return` on non-2xx status — and confirming
they fail, so they are pinning real behaviour.

The code is also clippy-clean (`cargo clippy --all-targets`).

---

## Design decisions and assumptions

### `getwalletinfo` no longer reports balances

The brief asks `wallet-info` to show a balance and an unconfirmed balance, which
reads like a straight mapping onto `getwalletinfo`. It is not: modern Bitcoin
Core no longer returns balance fields from that call. Verified against the node
this was built on (Core **30.0**), whose own `help getwalletinfo` lists neither
`balance` nor `unconfirmed_balance`.

So `wallet-info` makes **two** calls and folds them into one view:

| Field               | Source                            |
| ------------------- | --------------------------------- |
| Wallet name         | `getwalletinfo` → `walletname`    |
| Transactions        | `getwalletinfo` → `txcount`       |
| Balance             | `getbalances` → `mine.trusted`    |
| Unconfirmed balance | `getbalances` → `mine.untrusted_pending` |
| Immature balance    | `getbalances` → `mine.immature`   |

### Argument types in the generic `rpc` command

Bitcoin Core is strict about JSON argument types. `getblockhash "200"` is
rejected outright:

```
JSON value of type string is not of expected type number
```

...but `getblock <hash>` genuinely wants a string. Command-line arguments arrive
as text either way, so each one is offered to the JSON parser first and kept as a
string only if it is not valid JSON. `200` becomes a number, `true` a bool,
`[1,2]` an array, and a block hash stays a string.

The rule has one known corner: an argument that is **all digits** is always sent
as a number. Every identifier Core takes as a string — block hashes, txids,
addresses — is hex or bech32 and contains at least one non-digit in practice, so
this has not needed a workaround. Quoting is the escape hatch if it ever does:
`rpc somemethod '"200"'` sends the string `"200"`.

### Typed structs, with one deliberate exception

The four fixed commands deserialize into structs (`BlockchainInfo`,
`WalletInfo`, `Balances`), which is where the brief asks for strong typing. The
`rpc` passthrough necessarily stays `serde_json::Value` — it cannot know the
shape of a method chosen at runtime.

Both go through the *same* client method,
`RpcClient::call<T: DeserializeOwned>`, which is generic over the response type.
One code path owns transport, auth and error mapping; the caller decides how much
structure it wants back. Response structs also model only the fields that get
used, so a Core version that adds fields does not break them.

### Wallet RPCs always route through `/wallet/<name>`

Bitcoin Core exposes wallet methods at both `/` and `/wallet/<name>`. The root
endpoint works only while exactly one wallet is loaded, and starts failing with
error -19 the moment a second appears. The typed wallet commands therefore always
use the explicit path, which is why `balance` keeps working after
`createwallet alice`.

The generic `rpc` command deliberately does **not** do this: it routes through a
wallet only when `--wallet` is given, so it behaves like `bitcoin-cli` and stays
a faithful passthrough.

### Amounts are displayed, not computed

Core returns amounts as JSON numbers, so they arrive as `f64` and are formatted
to eight decimal places for display. That is fine for a read-only client, but
`f64` is the wrong type for money — anything doing arithmetic on balances should
carry integer satoshis instead. Nothing here does arithmetic on them.

### Errors are printed by hand

`main` returns `ExitCode` rather than `Result`. Returning a `Result` would make
Rust `Debug`-format the error, printing the enum variant next to the message.
Instead the error and its cause chain are printed to stderr and the process exits
1.

Error messages carry a `hint:` line naming the likely fix, because the five
failure modes the brief lists are nearly all setup mistakes — a node that is not
running, a password that does not match, a wallet that was never created.

### Difficulty formatting

Regtest difficulty is about `4.7e-10`, which spelled out in full is a wall of
leading zeroes. Core itself switches to scientific notation for values that
small, so this does too. Mainnet-scale difficulties still print plainly.
