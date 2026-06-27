# Troubleshooting Guide

Self-service reference for diagnosing and resolving common issues with the AuditLedger contract.

---

## Deployment Issues

### "WASM binary too large"

**Symptom:** `soroban contract deploy` rejects the WASM file or warns about size limits.

**Cause:** Debug symbols, unused dependencies, or a non-optimized release profile inflate binary size.

**Solution:**

1. Build with the release profile (the `Cargo.toml` already sets `opt-level = "z"`, `lto = true`, `debug = 0`, `strip = "symbols"`):
   ```bash
   cargo build --target wasm32-unknown-unknown --release
   ```
2. Verify the output size:
   ```bash
   ls -lh target/wasm32-unknown-unknown/release/audit_ledger.wasm
   ```
3. If still too large, run the Soroban optimizer:
   ```bash
   soroban contract optimize --wasm target/wasm32-unknown-unknown/release/audit_ledger.wasm
   # outputs audit_ledger.optimized.wasm
   ```
4. Check for unused feature flags or dev-only dependencies that leaked into the build.

---

### "Insufficient account balance"

**Symptom:** Transaction submission fails with `op_underfunded` or similar balance error.

**Cause:** The source account does not hold enough XLM to cover the base reserve plus transaction fee.

**Solution:**

- **Testnet:** Fund the account via Friendbot:
  ```bash
  curl "https://friendbot.stellar.org?addr=<YOUR_PUBLIC_KEY>"
  ```
- **Mainnet:** Transfer XLM to the source account from an exchange or another funded wallet.
- Verify the balance before deploying:
  ```bash
  soroban contract invoke --network testnet -- \
    soroban keys show <key-name>
  ```
- Stellar accounts require a minimum base reserve of 1 XLM (2 × 0.5 XLM). Each additional data entry adds 0.5 XLM to the reserve requirement.

---

### "Network timeout / transaction expired"

**Symptom:** CLI hangs or returns a timeout error; transaction is not included in a ledger.

**Cause:** Network congestion or a fee set too low for current surge pricing.

**Solution:**

1. Retry with a higher fee using `--fee`:
   ```bash
   soroban contract deploy \
     --wasm target/wasm32-unknown-unknown/release/audit_ledger.wasm \
     --source <secret_key> \
     --network testnet \
     --fee 100000
   ```
2. Check network status at [https://status.stellar.org](https://status.stellar.org).
3. Use a longer timeout if the RPC supports it, or increase the transaction's `timeBounds.maxTime`.

---

## Initialization Issues

### "Contract not found"

**Symptom:** Invocations return `contract not found` or similar errors.

**Cause:** Incorrect `CONTRACT_ID`, wrong network, or deployment did not complete.

**Solution:**

1. Confirm the contract ID matches what `deploy` printed and what is in your `.env`:
   ```bash
   grep CONTRACT_ID .env
   ```
2. Verify it exists on the correct network:
   ```bash
   soroban contract fetch --id <CONTRACT_ID> --network testnet
   ```
3. If the deploy failed mid-way, redeploy and reinitialize.

---

### "Owner authorization failed" / `CallerNotOwner`

**Symptom:** `initialize` or a governance function returns error code `1` (`CallerNotOwner`).

**Cause:** The `--source` key does not match the `owner` address passed to `initialize`, or `require_auth()` failed.

**Solution:**

1. Ensure the `--source` secret key corresponds to the same public key passed as `--owner`:
   ```bash
   soroban keys address <key-name>   # should match the --owner value
   ```
2. For `initialize`, the owner must sign the transaction (the SDK calls `owner.require_auth()` internally).
3. If using multi-sig, confirm all required signers are included.

---

## Runtime Issues

### `GlobalMaxLogsReached` (error code 2)

**Symptom:** `log_event` or `log_events` panics with error `2`.

**Cause:** `total_events` has reached `global_max_logs`.

**Solution (choose one):**

- **Increase the cap** (owner only):
  ```bash
  soroban contract invoke --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
    -- set_global_max_logs --caller $OWNER_ADDRESS --new_max 200000
  ```
- **Archive old events off-chain** using the backup scripts in `tools/backup/backup.sh`, then consider a contract migration if true pruning is needed.
- **Check the current state** before raising caps:
  ```bash
  soroban contract invoke --id $CONTRACT_ID --network testnet -- total_events
  ```

---

### `EventTypeMaxLogsReached` (error code 3)

**Symptom:** `log_event` panics with error `3` for a specific `event_type`.

**Cause:** The per-event-type log count has reached `EventCapConfig.max_logs` for that type.

**Solution (choose one):**

- **Increase the per-type cap** (owner only):
  ```bash
  soroban contract invoke --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
    -- set_event_max_logs \
    --caller $OWNER_ADDRESS \
    --event_type payment \
    --new_max 5000
  ```
- **Remove the cap entirely** (reverts to global-only enforcement):
  ```bash
  soroban contract invoke --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
    -- remove_event_cap \
    --caller $OWNER_ADDRESS \
    --event_type payment
  ```
- Note: `remove_event_cap` errors with `CapAlreadyRemoved` (17) if called a second time.

---

### `CallerNotOwner` (error code 1) at runtime

**Symptom:** A governance function (`set_global_max_logs`, `transfer_ownership`, etc.) returns error `1`.

**Cause:** The address passed as `caller` is not the current owner stored in contract state.

**Solution:**

1. Read the current owner:
   ```bash
   soroban contract invoke --id $CONTRACT_ID --network testnet -- get_owner
   ```
2. Re-invoke using the matching key for `--source` and `--caller`.
3. If ownership was transferred, use the new owner's key.

---

### `ContractPaused` (error code 13)

**Symptom:** All write operations are rejected.

**Cause:** The owner called `pause()` to freeze the contract.

**Solution:** Have the owner call `unpause()`:
```bash
soroban contract invoke --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
  -- unpause --caller $OWNER_ADDRESS
```

---

### `MetadataTooLarge` (error code 8)

**Symptom:** `log_event` fails with error `8`.

**Cause:** The `metadata` bytes exceed the effective cap (per-type or global; default 1 KB).

**Solution:**

- Trim the metadata payload on the client side before submitting.
- Or raise the metadata size cap (owner only):
  ```bash
  soroban contract invoke --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
    -- set_event_metadata_max_size \
    --caller $OWNER_ADDRESS \
    --event_type payment \
    --new_max 4096
  ```

---

### `RateLimitExceeded` (error code 14)

**Symptom:** A submitter's transaction is rejected with error `14`.

**Cause:** The submitter's per-ledger rate limit has been reached.

**Solution:**

- Wait for the next ledger (~5 seconds on testnet) and retry.
- Or have the owner increase the rate limit for the submitter:
  ```bash
  soroban contract invoke --id $CONTRACT_ID --source $OWNER_KEY --network testnet \
    -- set_submitter_rate_limit \
    --caller $OWNER_ADDRESS \
    --submitter $SUBMITTER_ADDRESS \
    --limit 10
  ```

---

## Integration Issues

### "Cannot parse event data"

**Symptom:** Off-chain consumers fail to decode the `metadata` field.

**Cause:** Metadata encoding does not match what the consumer expects (e.g., raw bytes vs. XDR-encoded struct vs. JSON string).

**Solution:**

1. Agree on a schema before logging. The `metadata` field is opaque `Bytes` — the contract does not enforce a format.
2. Recommended approach: encode metadata as XDR or JSON and document the schema in your integration.
3. To debug, read the raw bytes:
   ```bash
   soroban contract invoke --id $CONTRACT_ID --network testnet \
     -- get_event --id <EVENT_ID>
   ```
   Then decode the `metadata` field manually.
4. If using the JS or Python SDK, ensure you are calling `.toString()` or the appropriate decode helper on the returned `Bytes`.

---

### "Transaction too large"

**Symptom:** `log_events` (batch) transaction is rejected for exceeding the Stellar transaction size limit.

**Cause:** Batching too many events or too much metadata in a single transaction.

**Solution:**

1. Reduce the batch size. A safe starting point is 10–20 events per batch, depending on metadata size.
2. Split large batches into multiple calls:
   ```js
   const CHUNK_SIZE = 10;
   for (let i = 0; i < events.length; i += CHUNK_SIZE) {
     await client.logEvents(events.slice(i, i + CHUNK_SIZE));
   }
   ```
3. Use `log_event` (single) instead of `log_events` (batch) for very large metadata payloads.
4. Compress or truncate metadata off-chain and store the full payload in an off-chain store (IPFS, S3), logging only the content hash.

---

## FAQ

**Q: Can I delete or modify a logged event?**
A: No. The ledger is append-only by design. Events are content-addressed by SHA-256 hash and form a tamper-evident hash chain. Modifying history would require breaking SHA-256.

**Q: How do I read all events for a given type?**
A: Use `event_count` to get the total, then iterate with `get_event_by_type`:
```bash
COUNT=$(soroban contract invoke --id $CONTRACT_ID --network testnet -- event_count --event_type payment)
for i in $(seq 0 $((COUNT - 1))); do
  soroban contract invoke --id $CONTRACT_ID --network testnet \
    -- get_event_by_type --event_type payment --type_index $i
done
```

**Q: What happens if I call `initialize` twice?**
A: The second call panics with `AlreadyInitialized`. The contract is designed for a single initialization.

**Q: How do I transfer ownership if the current owner key is lost?**
A: There is no recovery path — ownership is enforced on-chain by `require_auth()`. Back up the owner's secret key securely. Consider setting up a multisig account as the owner.

**Q: The `get_event` call returns `EventDoesNotExist` (error 4). Why?**
A: The event ID is a SHA-256 hash computed from the event's fields plus the contract ID. Verify you are using the `BytesN<32>` ID returned by `log_event`, not a sequential index.

**Q: How do I check whether a specific event cap is active?**
A: Call `get_event_cap` for the event type. If the cap has been removed via `remove_event_cap`, only the global cap applies.
