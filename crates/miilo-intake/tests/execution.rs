use chrono::Utc;
use miilo_adapter_sdk::{
    intake::{ExecutionState, action},
    *,
};
use miilo_intake::{
    application as app,
    domain::Submission,
    execution::{self, Executor, Report, Request},
    storage::Files,
};
use std::cell::RefCell;
struct Backend {
    requests: RefCell<Vec<Request>>,
    fail: bool,
}
impl Executor for Backend {
    fn submit(&self, r: &Request) -> Result<Report, String> {
        self.requests.borrow_mut().push(r.clone());
        if self.fail {
            return Err("simulated timeout after backend may have accepted".into());
        }
        Ok(Report {
            job_id: r.job_id.clone(),
            execution: ExecutionState::ReadyForReview,
            summary: "Registered checks and independent review passed".into(),
            pr_url: Some("https://example.test/pull/1".into()),
        })
    }
}
fn accepted(store: &Files) {
    store
        .transaction(|s| {
            let now = Utc::now();
            app::import(
                s,
                Submission {
                    id: "task".into(),
                    title: "Test".into(),
                    summary: "Run checks".into(),
                    revision: "r1".into(),
                    workflow_id: "rust".into(),
                },
                now,
            )?;
            let revision = s.entries["task"].revision();
            app::decide(
                s,
                ActionInvocation {
                    protocol_version: "0.1".into(),
                    request_id: "one".into(),
                    provider_id: app::PROVIDER.into(),
                    action_id: action::ACCEPT.into(),
                    subject: SubjectReference {
                        kind: SubjectKind::Task,
                        id: "task".into(),
                    },
                    expected_revision: revision,
                    confirmation: Confirmation {
                        method: ConfirmationMethod::Manual,
                        confirmed_at: now,
                    },
                    input: serde_json::Map::new(),
                    idempotency_key: Some("one".into()),
                },
                now,
            )?;
            Ok(())
        })
        .unwrap();
}
#[test]
fn failed_submit_retries_the_exact_frozen_request_after_restart() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    let store = Files::new(&path).unwrap();
    accepted(&store);
    let unavailable = Backend {
        requests: RefCell::new(vec![]),
        fail: true,
    };
    execution::tick(&store, &unavailable, &["rust".into()], Utc::now()).unwrap();
    let recovered = Backend {
        requests: RefCell::new(vec![]),
        fail: false,
    };
    execution::tick(
        &Files::new(&path).unwrap(),
        &recovered,
        &["rust".into()],
        Utc::now(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&unavailable.requests.borrow()[0]).unwrap(),
        serde_json::to_value(&recovered.requests.borrow()[0]).unwrap()
    );
    store
        .transaction(|s| {
            assert_eq!(s.entries["task"].execution, ExecutionState::ReadyForReview);
            let snapshot = miilo_intake::projection::snapshot(s, Utc::now());
            assert_eq!(snapshot.work_items[0].provider_state, "ready_for_review");
            Ok(())
        })
        .unwrap();
    execution::tick(&store, &recovered, &["rust".into()], Utc::now()).unwrap();
    assert_eq!(recovered.requests.borrow().len(), 2);
    assert_eq!(
        recovered.requests.borrow()[0].job_id,
        recovered.requests.borrow()[1].job_id
    );
}
#[test]
fn unregistered_workflow_does_not_reach_an_executor() {
    let root = tempfile::tempdir().unwrap();
    let store = Files::new(&root.path().join("state")).unwrap();
    accepted(&store);
    let backend = Backend {
        requests: RefCell::new(vec![]),
        fail: false,
    };
    assert_eq!(
        execution::tick(&store, &backend, &["other".into()], Utc::now()).unwrap(),
        0
    );
    assert!(backend.requests.borrow().is_empty());
}
