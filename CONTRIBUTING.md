# Contributing to Decentralized Audit & Transparency Ledger

Thank you for your interest in contributing! This document covers everything you need to get started.

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](https://www.contributor-covenant.org/version/2/1/code_of_conduct/). By participating, you agree to uphold it. Report unacceptable behaviour to the maintainers.

---

## Getting Started

### Prerequisites

| Tool | Install |
|------|---------|
| Rust toolchain | [rustup.rs](https://rustup.rs/) |
| WASM target | `rustup target add wasm32-unknown-unknown` |
| Soroban CLI | `cargo install soroban-cli --features opt` |
| Docker & Compose | [docs.docker.com](https://docs.docker.com/get-docker/) |
| Node.js 20+ | [nodejs.org](https://nodejs.org/) (for UI / metrics exporter) |

### Fork & Clone

```bash
# 1. Fork the repository on GitHub, then:
git clone https://github.com/<your-username>/Decentralized-Audit-Transparency-Ledger.git
cd Decentralized-Audit-Transparency-Ledger

# 2. Add upstream remote
git remote add upstream https://github.com/daddygokings-art/Decentralized-Audit-Transparency-Ledger.git
```

### Build & Test

```bash
# Build the contract
cargo build

# Build the optimised WASM binary
cargo build --target wasm32-unknown-unknown --release

# Run the full test suite
cargo test

# Run a single test
cargo test test_log_event

# Format check (must pass before opening a PR)
cargo fmt --check

# Lint (zero warnings required)
cargo clippy -- -D warnings
```

### Local Docker Stack

```bash
cp .env.example .env          # configure environment variables
docker compose up --build     # start UI, metrics exporter, Prometheus, Grafana
```

- UI: http://localhost:3001
- Grafana: http://localhost:3000
- Prometheus: http://localhost:9090

---

## Coding Standards

### Rust Formatting

All Rust code **must** be formatted with `cargo fmt`. Run `cargo fmt` before committing; the CI pipeline enforces `cargo fmt --check`.

### Linting

Run `cargo clippy -- -D warnings`. Zero clippy warnings are required. Suppress a specific lint only when strictly necessary and document why with an inline comment.

### Naming Conventions

| Context | Convention |
|---------|-----------|
| Types, traits, enums | `PascalCase` |
| Functions, variables, modules | `snake_case` |
| Constants | `SCREAMING_SNAKE_CASE` |
| Contract storage keys | `PascalCase` enum variant (see `DataKey`) |

### Comments & Documentation

- Every `pub` function **must** have a doc comment (`///`) explaining what it does, its parameters, and any errors it can return.
- Inline comments (`//`) should explain *why*, not *what*.
- Keep comments up-to-date when you change the code.

---

## Testing Requirements

- All new public functions must have at least one unit test in `src/test.rs`.
- Edge cases (zero caps, overflow, access control) must be covered.
- Run `cargo test` before opening a PR — all tests must pass.
- The project targets **90% test coverage**. Use `cargo tarpaulin` locally to check your contribution does not reduce coverage.

```bash
# Install tarpaulin (first time only)
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html
```

---

## PR Workflow

### Branch Naming

```
feature/<short-description>     # new functionality
bugfix/<short-description>      # bug fixes
docs/<short-description>        # documentation only
refactor/<short-description>    # code restructuring without behaviour change
test/<short-description>        # test additions or fixes
```

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add rate-limit enforcement per submitter
fix: prevent integer overflow in log_events batch
docs: add deployment guide for mainnet
test: cover zero global_max_logs edge case
```

### PR Checklist

Before opening your pull request, confirm all of the following:

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes (all tests green)
- [ ] New functions have doc comments
- [ ] New behaviour has test coverage
- [ ] The PR description explains *what* changed and *why*
- [ ] No secrets, `.env` files, or generated artefacts are committed

### Review Process

1. Open a pull request against `master` (or the active feature branch).
2. At least **1 maintainer approval** is required before merging.
3. Address all review comments; re-request review once resolved.
4. The maintainer will squash-merge once CI is green and approval is given.
5. Delete your branch after merging.

---

## Issue Tracking & Bounty Program

### Claiming an Issue

1. Find an open issue labelled `bounty` or `good first issue`.
2. Comment "I'd like to work on this" — a maintainer will assign it to you.
3. If you have a new idea, open an issue first and wait for a maintainer to confirm scope before starting work.

### Bounty Points

| Difficulty | Points | Example |
|------------|--------|---------|
| High | 200 | Implement global vs. per-event logging limits |
| Medium | 150 | Write edge-case boundary tests |
| Trivial | 100 | Standardise metadata structure |

Points are tracked per GitHub account and redeemable for rewards as announced by the project maintainers.

### Proposing New Work

Submit a proposal issue with:
- **Problem**: what you want to solve
- **Approach**: how you plan to solve it
- **Estimated scope**: lines of code, test count, files affected

Wait for maintainer sign-off before implementing.

---

## Questions

Open a [GitHub Discussion](../../discussions) or comment on the relevant issue. Please do not use issue comments for general questions unrelated to the issue.
