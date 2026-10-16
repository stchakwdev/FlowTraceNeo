//! [`InMemoryBackend`] against the backend contract (`tests/common`), the whole
//! `commands-accepted.json` log, and the module's refusal texts.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use common::{backend_contract, command, demo_org_id, demo_state, fixture, new_vendor};
use ledger_client::{
    BackendEvent, ClientError, ConnectionStatus, IN_MEMORY_IDENTITY, InMemoryBackend, LedgerBackend,
};
use ledger_core::CommandResult;
use ledger_core::types::{Actor, LedgerState};

#[test]
fn the_in_memory_backend_keeps_the_contract() {
    let backend = InMemoryBackend::demo().unwrap();
    let run = backend_contract(&backend, &demo_org_id());
    // The commands env draws fx- ids, which the demo books (demo-...) never use.
    assert_eq!(run.vendor_id, "fx-0001");
}

#[test]
fn the_demo_backend_holds_the_demo_books_as_the_demo_user() {
    let backend = InMemoryBackend::demo().unwrap();
    assert_eq!(backend.state(&demo_org_id()).unwrap(), demo_state());
    let orgs = backend.organisations();
    assert_eq!(orgs.len(), 1);
    assert_eq!(
        (
            orgs[0].members[0].user_id.as_str(),
            orgs[0].members[0].identity.as_str()
        ),
        ("user_dummy", IN_MEMORY_IDENTITY)
    );
    assert_eq!(
        backend.status(),
        ConnectionStatus::Ready {
            identity: IN_MEMORY_IDENTITY.to_owned()
        }
    );
}

#[test]
fn the_whole_accepted_command_log_replays_to_the_fixture_books() {
    let f = fixture("commands-accepted");
    assert_eq!(f["initial_state"], "demo-books.json#/state");
    let backend = InMemoryBackend::demo().unwrap();
    let org = demo_org_id();
    let steps = f["steps"].as_array().unwrap();
    for (i, step) in steps.iter().enumerate() {
        let actor: Actor = serde_json::from_value(step["actor"].clone()).unwrap();
        backend.set_actor(actor);
        let result = backend
            .submit(&org, command(&step["command"]))
            .wait()
            .unwrap_or_else(|e| panic!("step {i} {}: {e}", step["command"]["type"]));
        let expected: CommandResult = serde_json::from_value(step["result"].clone()).unwrap();
        assert_eq!(result, expected, "step {i} {}", step["command"]["type"]);
    }
    let final_state: LedgerState = serde_json::from_value(f["final_state"].clone()).unwrap();
    assert_eq!(backend.state(&org).unwrap(), final_state);
    assert_eq!(steps.len(), 37);
}

#[test]
fn another_organisation_has_no_books_here() {
    let backend = InMemoryBackend::demo().unwrap();
    assert!(
        matches!(backend.state("other-org"), Err(ClientError::NoLedger(o)) if o == "other-org")
    );
    assert!(matches!(
        backend.submit("other-org", new_vendor()).wait(),
        Err(ClientError::NoLedger(_))
    ));
}

#[test]
fn a_member_without_permission_gets_the_engine_refusal() {
    let backend = InMemoryBackend::demo().unwrap();
    let officer: Actor =
        serde_json::from_value(fixture("commands-accepted")["actors"]["officer"].clone()).unwrap();
    backend.set_actor(officer);
    let refused = backend
        .submit(&demo_org_id(), common::unbalanced_entry())
        .wait();
    let Err(ClientError::Ledger(e)) = refused else {
        panic!("came back {refused:?}");
    };
    assert_eq!(e.code(), ledger_core::IssueCode::Permission, "{e}");
}

#[test]
fn every_subscriber_gets_the_status_then_each_change() {
    let backend = InMemoryBackend::demo().unwrap();
    let (a, b) = (backend.subscribe(), backend.subscribe());
    backend.submit(&demo_org_id(), new_vendor()).wait().unwrap();
    for rx in [a, b] {
        let events = common::queued(&rx);
        assert_eq!(events.len(), 2, "{events:?}");
        assert!(matches!(
            events[0],
            BackendEvent::Status(ConnectionStatus::Ready { .. })
        ));
        assert!(
            matches!(&events[1], BackendEvent::Changed { orgs } if orgs.contains(&demo_org_id()))
        );
    }
}

#[test]
fn a_dropped_subscriber_is_forgotten() {
    let backend = InMemoryBackend::demo().unwrap();
    drop(backend.subscribe());
    let kept = backend.subscribe();
    backend.submit(&demo_org_id(), new_vendor()).wait().unwrap();
    assert_eq!(common::queued(&kept).len(), 2);
}

#[test]
fn the_ticket_also_resolves_when_awaited() {
    let backend = InMemoryBackend::demo().unwrap();
    let ticket = backend.submit(&demo_org_id(), new_vendor());
    let result = block_on(ticket.result()).unwrap();
    assert!(matches!(result, CommandResult::Id { .. }));
}

/// A minimal executor for one ready future: polls it with a no-op waker.
fn block_on<F: std::future::Future>(f: F) -> F::Output {
    use std::task::{Context, Poll, Waker};
    let mut f = std::pin::pin!(f);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(out) = f.as_mut().poll(&mut cx) {
            return out;
        }
        std::thread::yield_now();
    }
}
