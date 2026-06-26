# Integration Testing Guide

How to validate AuditLedger contract behaviour against the real Stellar testnet network, beyond simulated unit tests.

---

## Prerequisites

1. **Stellar testnet account funded via Friendbot:**
   ```bash
   # Generate a new keypair
   soroban keys generate --network testnet test-owner
   soroban keys address test-owner   # copy the public key

   # Fund via Friendbot
   curl "https://friendbot.stellar.org?addr=$(soroban keys address test-owner)"
   ```

2. **Soroban CLI configured for testnet:**
   ```bash
   soroban network add testnet \
     --rpc-url https://soroban-testnet.stellar.org \
     --network-passphrase "Test SDF Network ; September 2015"
   ```
   Verify:
   ```bash
   soroban network ls
   ```

3. **WASM built and ready:**
   ```bash
   cargo build --target wasm32-unknown-unknown --release
   ```

4. **Environment variables set** (copy `.env.example` → `.env`):
   ```
   CONTRACT_ID=<deployed_contract_id>
   OWNER_ADDRESS=<owner_public_key>
   OWNER_KEY=test-owner          # soroban key name
   RPC_URL=https://soroban-testnet.stellar.org
   NETWORK=testnet
   ```

---

## Test Environment Setup

### Deploy and initialize

```bash
# Deploy
CONTRACT_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/audit_ledger.wasm \
  --source $OWNER_KEY \
  --network testnet)
echo "CONTRACT_ID=$CONTRACT_ID"

# Initialize
soroban contract invoke \
  --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
  -- initialize \
  --owner $(soroban keys address $OWNER_KEY) \
  --global_max_logs 10000
```

Verify:
```bash
soroban contract invoke --id $CONTRACT_ID --network testnet -- total_events
# Expected: 0
```

---

## Integration Test Scenarios

### 1. End-to-end event logging flow

Tests a full round-trip: log an event, read it back, verify all fields.

```bash
# Log one event
EVENT_ID=$(soroban contract invoke \
  --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
  -- log_event \
  --submitter $(soroban keys address $OWNER_KEY) \
  --event_type payment \
  --metadata "$(echo -n '{"amount":100}' | xxd -p)")

echo "Event ID: $EVENT_ID"

# Verify count
soroban contract invoke --id $CONTRACT_ID --network testnet -- total_events
# Expected: 1

# Read event back
soroban contract invoke --id $CONTRACT_ID --network testnet \
  -- get_event --id "$EVENT_ID"
```

Expected: returned `Event` has `index=0`, correct `event_type`, `submitter`, and `metadata`.

---

### 2. Multi-submitter concurrent logging

Tests that independent submitters can each log events and that indices are sequential.

```bash
soroban keys generate --network testnet submitter-a
soroban keys generate --network testnet submitter-b
curl "https://friendbot.stellar.org?addr=$(soroban keys address submitter-a)"
curl "https://friendbot.stellar.org?addr=$(soroban keys address submitter-b)"

# Each logs one event
ID_A=$(soroban contract invoke \
  --id $CONTRACT_ID --source submitter-a --network testnet \
  -- log_event \
  --submitter $(soroban keys address submitter-a) \
  --event_type transfer --metadata "YQ==")

ID_B=$(soroban contract invoke \
  --id $CONTRACT_ID --source submitter-b --network testnet \
  -- log_event \
  --submitter $(soroban keys address submitter-b) \
  --event_type transfer --metadata "Yg==")

# Total should be 3 (1 from scenario 1 + 2 here)
soroban contract invoke --id $CONTRACT_ID --network testnet -- total_events

# Verify per-type count
soroban contract invoke --id $CONTRACT_ID --network testnet \
  -- event_count --event_type transfer
# Expected: 2
```

---

### 3. Governance operations

Tests that owner-only functions are gated and work correctly.

```bash
OWNER=$(soroban keys address $OWNER_KEY)

# Set a per-event cap
soroban contract invoke \
  --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
  -- set_event_max_logs \
  --caller $OWNER \
  --event_type payment \
  --new_max 50

# Attempt governance call from non-owner — must fail with error 1
soroban contract invoke \
  --id $CONTRACT_ID --source submitter-a --network testnet \
  -- set_global_max_logs \
  --caller $(soroban keys address submitter-a) \
  --new_max 999 \
  && echo "FAIL: should have been rejected" || echo "PASS: rejected as expected"
```

---

### 4. Cap management

Tests set → enforce → remove cycle for per-event caps.

```bash
OWNER=$(soroban keys address $OWNER_KEY)

# Set cap to 1 for 'refund'
soroban contract invoke \
  --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
  -- set_event_max_logs --caller $OWNER --event_type refund --new_max 1

# Log one refund (should succeed)
soroban contract invoke \
  --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
  -- log_event --submitter $OWNER --event_type refund --metadata "Zmlyc3Q="

# Log second refund (should fail with error 3)
soroban contract invoke \
  --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
  -- log_event --submitter $OWNER --event_type refund --metadata "c2Vjb25k" \
  && echo "FAIL: should have been capped" || echo "PASS: cap enforced"

# Remove cap and log again (should succeed)
soroban contract invoke \
  --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
  -- remove_event_cap --caller $OWNER --event_type refund

soroban contract invoke \
  --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
  -- log_event --submitter $OWNER --event_type refund --metadata "dGhpcmQ="
echo "PASS: cap removed, logging resumed"
```

---

### 5. Ownership transfer

Tests that ownership transfers correctly and the old owner loses governance access.

```bash
soroban keys generate --network testnet new-owner
curl "https://friendbot.stellar.org?addr=$(soroban keys address new-owner)"

OLD_OWNER=$(soroban keys address $OWNER_KEY)
NEW_OWNER=$(soroban keys address new-owner)

# Transfer
soroban contract invoke \
  --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
  -- transfer_ownership --caller $OLD_OWNER --new_owner $NEW_OWNER

# Old owner should now be rejected
soroban contract invoke \
  --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
  -- set_global_max_logs --caller $OLD_OWNER --new_max 9999 \
  && echo "FAIL" || echo "PASS: old owner rejected"

# New owner should succeed
soroban contract invoke \
  --id $CONTRACT_ID --source new-owner --network testnet \
  -- set_global_max_logs --caller $NEW_OWNER --new_max 9999
echo "PASS: new owner accepted"
```

---

## Test Scripts

### Bash — full smoke suite

```bash
#!/usr/bin/env bash
# scripts/integration_smoke.sh
set -euo pipefail
source .env

OWNER=$(soroban keys address $OWNER_KEY)

run() { echo ">>> $*"; soroban contract invoke --id $CONTRACT_ID --source $OWNER_KEY --network $NETWORK -- "$@"; }

echo "=== 1. total_events before ==="
run total_events

echo "=== 2. log_event ==="
ID=$(run log_event --submitter $OWNER --event_type smoke --metadata "dGVzdA==")
echo "Event ID: $ID"

echo "=== 3. total_events after ==="
run total_events

echo "=== 4. get_event ==="
run get_event --id "$ID"

echo "=== 5. event_count by type ==="
run event_count --event_type smoke

echo "All smoke tests passed."
```

Run with:
```bash
bash scripts/integration_smoke.sh
```

### Node.js — using `@stellar/stellar-sdk`

```js
// scripts/integration.mjs
import { Contract, Keypair, Networks, SorobanRpc, TransactionBuilder, BASE_FEE, nativeToScVal } from '@stellar/stellar-sdk';

const RPC_URL = process.env.RPC_URL ?? 'https://soroban-testnet.stellar.org';
const CONTRACT_ID = process.env.CONTRACT_ID;
const SECRET = process.env.OWNER_SECRET;

const server = new SorobanRpc.Server(RPC_URL);
const keypair = Keypair.fromSecret(SECRET);
const account = await server.getAccount(keypair.publicKey());

async function invoke(method, args) {
  const contract = new Contract(CONTRACT_ID);
  const tx = new TransactionBuilder(account, { fee: BASE_FEE, networkPassphrase: Networks.TESTNET })
    .addOperation(contract.call(method, ...args))
    .setTimeout(30)
    .build();
  const prepared = await server.prepareTransaction(tx);
  prepared.sign(keypair);
  const result = await server.sendTransaction(prepared);
  // poll for completion
  let response;
  do {
    await new Promise(r => setTimeout(r, 3000));
    response = await server.getTransaction(result.hash);
  } while (response.status === 'NOT_FOUND');
  if (response.status !== 'SUCCESS') throw new Error(`Transaction failed: ${JSON.stringify(response)}`);
  return response.returnValue;
}

// Log one event
const eventId = await invoke('log_event', [
  nativeToScVal(keypair.publicKey(), { type: 'address' }),
  nativeToScVal('payment', { type: 'symbol' }),
  nativeToScVal(Buffer.from('{"amount":42}'), { type: 'bytes' }),
]);
console.log('Logged event ID:', eventId);

// Read it back
const event = await invoke('get_event', [eventId]);
console.log('Event:', event);
```

Run with:
```bash
OWNER_SECRET=<secret> CONTRACT_ID=<id> node scripts/integration.mjs
```

---

## Continuous Integration

### Testnet tests in GitHub Actions

Add a separate job in `.github/workflows/test.yml` that runs only on `main` or manually:

```yaml
integration:
  runs-on: ubuntu-latest
  if: github.ref == 'refs/heads/main' || github.event_name == 'workflow_dispatch'
  env:
    OWNER_SECRET: ${{ secrets.TESTNET_OWNER_SECRET }}
    CONTRACT_ID: ${{ secrets.TESTNET_CONTRACT_ID }}
    RPC_URL: https://soroban-testnet.stellar.org
    NETWORK: testnet
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - name: Install Soroban CLI
      run: cargo install soroban-cli --features opt --locked
    - name: Run integration smoke tests
      run: bash scripts/integration_smoke.sh
```

Store `TESTNET_OWNER_SECRET` and `TESTNET_CONTRACT_ID` as repository secrets, not in source code.

### Testnet vs. standalone network for CI

| Approach | Pros | Cons |
|----------|------|------|
| **Testnet** | Real network conditions, Friendbot funding, no setup | Flaky if testnet is down; slower (~5 s/ledger) |
| **Standalone / local node** | Fast, deterministic, no external dependency | Requires Docker; more setup; not identical to mainnet |

Recommended strategy:
- Unit tests (`cargo test`) on every PR — fast, no network.
- Standalone network tests on every PR using `docker compose up` with the Stellar quickstart image.
- Testnet integration tests on merge to `main` only.

---

## Best Practices

### Test data cleanup

Testnet state is persistent. To avoid cross-test pollution:
- Deploy a **fresh contract per test run** using a disposable keypair funded by Friendbot.
- Record the `CONTRACT_ID` at the start of the run and discard it afterward.
- Never reuse a testnet contract across unrelated test suites.

### Error handling in integration tests

- Wrap every `soroban contract invoke` call in error checks (`set -e` in bash, try/catch in Node.js).
- Parse the error code from the returned XDR to assert the *expected* error in negative tests:
  ```bash
  output=$(soroban contract invoke ... 2>&1) || {
    echo "$output" | grep "Error(Contract, #3)" && echo "PASS: EventTypeMaxLogsReached"
  }
  ```
- Add timeouts to all RPC polling loops to prevent hanging CI jobs.

### Reporting test results

- Use `set -euo pipefail` and emit a summary line (`PASS` / `FAIL`) after each scenario.
- In CI, use GitHub Actions' `::error::` annotation syntax for failures:
  ```bash
  echo "::error::Integration test failed: $scenario"
  ```
- Capture `soroban contract invoke` output with `--output json` for structured parsing and archiving.
