use super::*;
use soroban_sdk::testutils::{Address as _, Events, Ledger};
use soroban_sdk::{symbol_short, Bytes, Env};

fn create_ledger() -> (Env, Address, AuditLedgerClient<'static>) {
    let env = Env::default();
    let owner = Address::generate(&env);
    let contract_id = env.register(AuditLedger, ());
    let client = AuditLedgerClient::new(&env, &contract_id);

    env.mock_all_auths();
    client.initialize(&owner, &100);
    (env, owner, client)
}

#[test]
fn test_initialize() {
    let env = Env::default();
    let owner = Address::generate(&env);
    let contract_id = env.register(AuditLedger, ());
    let client = AuditLedgerClient::new(&env, &contract_id);

    env.mock_all_auths();
    client.initialize(&owner, &100);

    assert_eq!(client.total_events(), 0);
}

#[test]
fn test_log_event() {
    let (_env, _owner, client) = create_ledger();
    let submitter = Address::generate(&_env);
    let event_type = symbol_short!("payment");

    _env.mock_all_auths();
    let idx = client.log_event(&submitter, &event_type, &Bytes::from_slice(&_env, b"tx1"));

    assert_eq!(idx, 0);
    assert_eq!(client.total_events(), 1);

    let evt = client.get_event(&0);
    assert_eq!(evt.index, 0);
    assert_eq!(evt.event_type, event_type);
    assert_eq!(evt.submitter, submitter);
    assert_eq!(evt.metadata, Bytes::from_slice(&_env, b"tx1"));
}

#[test]
fn test_log_multiple_events() {
    let (_env, _owner, client) = create_ledger();
    let submitter = Address::generate(&_env);
    let payment = symbol_short!("payment");
    let refund = symbol_short!("refund");

    _env.mock_all_auths();
    client.log_event(&submitter, &payment, &Bytes::from_slice(&_env, b"tx1"));
    client.log_event(&submitter, &payment, &Bytes::from_slice(&_env, b"tx2"));
    client.log_event(&submitter, &refund, &Bytes::from_slice(&_env, b"tx3"));

    assert_eq!(client.total_events(), 3);
    assert_eq!(client.event_count(&payment), 2);
    assert_eq!(client.event_count(&refund), 1);

    let evt0 = client.get_event_by_type(&payment, &0);
    assert_eq!(evt0.metadata, Bytes::from_slice(&_env, b"tx1"));

    let evt1 = client.get_event_by_type(&payment, &1);
    assert_eq!(evt1.metadata, Bytes::from_slice(&_env, b"tx2"));

    let evt2 = client.get_event_by_type(&refund, &0);
    assert_eq!(evt2.metadata, Bytes::from_slice(&_env, b"tx3"));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #4)")]
fn test_get_nonexistent_event_panics() {
    let (_env, _owner, client) = create_ledger();
    client.get_event(&999);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")]
fn test_get_event_by_type_out_of_bounds_panics() {
    let (_env, _owner, client) = create_ledger();
    let payment = symbol_short!("payment");
    client.get_event_by_type(&payment, &0);
}

#[test]
fn test_per_event_max_logs() {
    let (_env, _owner, client) = create_ledger();
    let submitter = Address::generate(&_env);
    let payment = symbol_short!("payment");

    _env.mock_all_auths();
    client.set_event_max_logs(&_owner, &payment, &2);

    client.log_event(&submitter, &payment, &Bytes::from_slice(&_env, b"tx1"));
    client.log_event(&submitter, &payment, &Bytes::from_slice(&_env, b"tx2"));
    assert_eq!(client.event_count(&payment), 2);

    let result = client.try_log_event(&submitter, &payment, &Bytes::from_slice(&_env, b"tx3"));
    assert!(result.is_err());
}

#[test]
fn test_global_max_logs() {
    let env = Env::default();
    let owner = Address::generate(&env);
    let submitter = Address::generate(&env);
    let contract_id = env.register(AuditLedger, ());
    let client = AuditLedgerClient::new(&env, &contract_id);

    env.mock_all_auths();
    client.initialize(&owner, &2);

    let payment = symbol_short!("payment");
    let refund = symbol_short!("refund");

    client.log_event(&submitter, &payment, &Bytes::from_slice(&env, b"tx1"));
    client.log_event(&submitter, &refund, &Bytes::from_slice(&env, b"tx2"));

    let result = client.try_log_event(&submitter, &payment, &Bytes::from_slice(&env, b"tx3"));
    assert!(result.is_err());
}

#[test]
fn test_owner_can_set_global_max_logs() {
    let (_env, _owner, client) = create_ledger();
    _env.mock_all_auths();
    client.set_global_max_logs(&_owner, &200);
    assert_eq!(client.total_events(), 0);
}

#[test]
fn test_non_owner_cannot_set_global_max() {
    let (_env, _owner, client) = create_ledger();
    let attacker = Address::generate(&_env);

    _env.mock_all_auths();
    let result = client.try_set_global_max_logs(&attacker, &200);
    assert!(result.is_err());
}

#[test]
fn test_transfer_ownership() {
    let (_env, _owner, client) = create_ledger();
    let new_owner = Address::generate(&_env);

    _env.mock_all_auths();
    client.transfer_ownership(&_owner, &new_owner);

    client.set_global_max_logs(&new_owner, &300);
}

#[test]
fn test_remove_event_cap() {
    let (_env, _owner, client) = create_ledger();
    let payment = symbol_short!("payment");

    _env.mock_all_auths();
    client.set_event_max_logs(&_owner, &payment, &5);
    client.remove_event_cap(&_owner, &payment);
}

#[test]
fn test_zero_global_max_logs() {
    let env = Env::default();
    let owner = Address::generate(&env);
    let submitter = Address::generate(&env);
    let contract_id = env.register(AuditLedger, ());
    let client = AuditLedgerClient::new(&env, &contract_id);

    env.mock_all_auths();
    client.initialize(&owner, &0);

    let result = client.try_log_event(
        &submitter,
        &symbol_short!("p"),
        &Bytes::from_slice(&env, b"x"),
    );
    assert!(result.is_err());
}

#[test]
fn test_set_global_max_to_zero_after_events() {
    let (_env, _owner, client) = create_ledger();
    let submitter = Address::generate(&_env);

    _env.mock_all_auths();
    client.log_event(
        &submitter,
        &symbol_short!("p"),
        &Bytes::from_slice(&_env, b"tx1"),
    );
    client.set_global_max_logs(&_owner, &0);

    let result = client.try_log_event(
        &submitter,
        &symbol_short!("p"),
        &Bytes::from_slice(&_env, b"tx2"),
    );
    assert!(result.is_err());
}

#[test]
fn test_zero_event_max_logs() {
    let (_env, _owner, client) = create_ledger();
    let payment = symbol_short!("payment");
    let submitter = Address::generate(&_env);

    _env.mock_all_auths();
    client.set_event_max_logs(&_owner, &payment, &0);

    let result = client.try_log_event(&submitter, &payment, &Bytes::from_slice(&_env, b"tx1"));
    assert!(result.is_err());
}

#[test]
fn test_set_event_max_equal_to_current_count() {
    let (_env, _owner, client) = create_ledger();
    let payment = symbol_short!("payment");
    let submitter = Address::generate(&_env);

    _env.mock_all_auths();
    client.log_event(&submitter, &payment, &Bytes::from_slice(&_env, b"tx1"));
    client.log_event(&submitter, &payment, &Bytes::from_slice(&_env, b"tx2"));

    client.set_event_max_logs(&_owner, &payment, &2);

    let result = client.try_log_event(&submitter, &payment, &Bytes::from_slice(&_env, b"tx3"));
    assert!(result.is_err());
}

#[test]
fn test_event_was_emitted() {
    let (_env, _owner, client) = create_ledger();
    let submitter = Address::generate(&_env);
    let payment = symbol_short!("payment");
    let meta = Bytes::from_slice(&_env, b"emit-test");

    _env.mock_all_auths();
    client.log_event(&submitter, &payment, &meta);

    let contract_events = _env.events().all();
    let events = contract_events.events();
    assert!(!events.is_empty());
}

#[test]
fn test_log_event_with_empty_metadata() {
    let (_env, _owner, client) = create_ledger();
    let submitter = Address::generate(&_env);
    let payment = symbol_short!("payment");

    _env.mock_all_auths();
    let idx = client.log_event(&submitter, &payment, &Bytes::new(&_env));

    let evt = client.get_event(&idx);
    assert_eq!(evt.metadata.len(), 0);
}

#[test]
fn test_multiple_event_types_independent() {
    let (_env, _owner, client) = create_ledger();
    let submitter = Address::generate(&_env);
    let type_a = symbol_short!("type_a");
    let type_b = symbol_short!("type_b");

    _env.mock_all_auths();
    client.set_event_max_logs(&_owner, &type_a, &1);
    client.set_event_max_logs(&_owner, &type_b, &1);

    client.log_event(&submitter, &type_a, &Bytes::from_slice(&_env, b"a1"));
    client.log_event(&submitter, &type_b, &Bytes::from_slice(&_env, b"b1"));

    let result = client.try_log_event(&submitter, &type_a, &Bytes::from_slice(&_env, b"a2"));
    assert!(result.is_err());
}

#[test]
fn test_log_event_returns_correct_fields() {
    let (_env, _owner, client) = create_ledger();
    let submitter = Address::generate(&_env);
    let payment = symbol_short!("payment");
    let meta = Bytes::from_slice(&_env, b"test-meta");

    _env.ledger().set_timestamp(1000);
    _env.mock_all_auths();
    let idx = client.log_event(&submitter, &payment, &meta);
    let evt = client.get_event(&idx);

    assert_eq!(evt.index, 0);
    assert_eq!(evt.event_type, payment);
    assert_eq!(evt.submitter, submitter);
    assert_eq!(evt.metadata, meta);
    assert_eq!(evt.timestamp, 1000);
}

#[test]
fn test_set_global_max_equal_to_current_count() {
    let (_env, _owner, client) = create_ledger();
    let submitter = Address::generate(&_env);
    let payment = symbol_short!("payment");

    _env.mock_all_auths();
    client.log_event(&submitter, &payment, &Bytes::from_slice(&_env, b"tx1"));
    client.log_event(&submitter, &payment, &Bytes::from_slice(&_env, b"tx2"));

    client.set_global_max_logs(&_owner, &2);

    let result = client.try_log_event(&submitter, &payment, &Bytes::from_slice(&_env, b"tx3"));
    assert!(result.is_err());
}

#[test]
fn test_remove_cap_then_unlimited() {
    let (_env, _owner, client) = create_ledger();
    let payment = symbol_short!("payment");
    let submitter = Address::generate(&_env);

    _env.mock_all_auths();
    client.set_event_max_logs(&_owner, &payment, &0);

    let result = client.try_log_event(&submitter, &payment, &Bytes::from_slice(&_env, b"blocked"));
    assert!(result.is_err());

    client.remove_event_cap(&_owner, &payment);

    client.log_event(
        &submitter,
        &payment,
        &Bytes::from_slice(&_env, b"now-unblocked"),
    );
    assert_eq!(client.event_count(&payment), 1);
}

#[test]
fn test_mixed_types_with_limits() {
    let (_env, _owner, client) = create_ledger();
    let submitter = Address::generate(&_env);
    let type_a = symbol_short!("TypeA");
    let type_b = symbol_short!("TypeB");
    let type_c = symbol_short!("TypeC");

    _env.mock_all_auths();
    client.set_event_max_logs(&_owner, &type_a, &2);
    client.set_event_max_logs(&_owner, &type_b, &3);

    client.log_event(&submitter, &type_a, &Bytes::from_slice(&_env, b"a1"));
    client.log_event(&submitter, &type_a, &Bytes::from_slice(&_env, b"a2"));
    client.log_event(&submitter, &type_b, &Bytes::from_slice(&_env, b"b1"));
    client.log_event(&submitter, &type_b, &Bytes::from_slice(&_env, b"b2"));
    client.log_event(&submitter, &type_b, &Bytes::from_slice(&_env, b"b3"));
    client.log_event(&submitter, &type_c, &Bytes::from_slice(&_env, b"c1"));

    assert_eq!(client.total_events(), 6);
    assert_eq!(client.event_count(&type_a), 2);
    assert_eq!(client.event_count(&type_b), 3);
    assert_eq!(client.event_count(&type_c), 1);

    let result = client.try_log_event(&submitter, &type_a, &Bytes::from_slice(&_env, b"a3"));
    assert!(result.is_err());
}
