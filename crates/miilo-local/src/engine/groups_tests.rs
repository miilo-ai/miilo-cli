use super::*;
#[test]
fn patterns_are_literal_except_star_and_question_mark() {
    assert!(wildcard("vave-*", "vave-ai"));
    assert!(wildcard("miilo-?e", "miilo-ce"));
    assert!(!wildcard("vave-*", "miilo-ce"));
    assert!(!wildcard("[ab]", "a"));
    assert!(wildcard("*", ""));
    assert!(!wildcard("?", ""));
}
#[test]
fn folders_are_scoped_and_nested_projects_match() {
    let temporary = tempfile::tempdir().unwrap();
    let base = fs::canonicalize(temporary.path()).unwrap();
    let work = base.join("Work");
    let project = work.join("vave-ai/src");
    let other = base.join("Work-other/vave-ai");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&other).unwrap();
    let rule =
        json!({"name":"Vave","folder":work,"pattern":"vave-*","kind":"codex","future":false});
    assert!(validate(&json!([rule.clone()])).is_ok());
    let session = json!({"kind":"codex","observable":true,"cwd":project});
    assert!(matches(&rule, &session));
    assert!(!matches(
        &rule,
        &json!({"cwd":other,"kind":"codex","observable":true})
    ));
    let link = work.join("vave-link");
    std::os::unix::fs::symlink(&other, &link).unwrap();
    assert!(!matches(
        &rule,
        &json!({"cwd":link,"kind":"codex","observable":true})
    ));
    let encoded = project
        .to_string_lossy()
        .trim_start_matches('/')
        .replace('/', "-");
    assert_eq!(cursor_workspace(&work, &encoded), Some(project.clone()));
    assert_eq!(cursor_workspace(&other, &encoded), None);
    assert!(!matches(
        &rule,
        &json!({"cwd":project,"kind":"cursor","observable":true})
    ));
    assert!(!matches(
        &rule,
        &json!({"cwd":project,"kind":"codex","observable":false})
    ));
    assert!(validate(&json!([rule.clone(), rule.clone()])).is_err());
    let preview = preview(&[session], &[rule]);
    assert_eq!(
        preview["groups"][0]["sessions"].as_array().unwrap().len(),
        1
    );
}

#[test]
fn future_enrollment_requires_new_creation_and_server_bound_consent() {
    let temporary = tempfile::tempdir().unwrap();
    let folder = temporary.path().canonicalize().unwrap();
    let now = chrono::Utc::now();
    let cutoff = (now - chrono::Duration::hours(1)).to_rfc3339();
    let rule = json!({"name":"Work","folder":folder,"pattern":"","kind":"codex","future":true,"enrolledAfter":cutoff});
    let mut session = json!({"kind":"codex","cwd":folder,"observable":true,"createdAt":(now-chrono::Duration::hours(2)).to_rfc3339()});
    assert!(!future_matches(&rule, &session));
    session["createdAt"] = json!((now - chrono::Duration::minutes(1)).to_rfc3339());
    assert!(future_matches(&rule, &session));
    session["createdAt"] = json!(now.timestamp_millis());
    assert!(!future_matches(&rule, &session));
    session["createdAt"] = Value::Null;
    assert!(!future_matches(&rule, &session));
    let mut rules = vec![rule.clone()];
    bind_consent(&mut rules, &json!([rule]));
    assert_eq!(rules[0]["enrolledAfter"], cutoff);
    let mut forged = rules.clone();
    forged[0]["enrolledAfter"] = json!("1900-01-01T00:00:00Z");
    bind_consent(&mut forged, &Value::Null);
    assert_ne!(forged[0]["enrolledAfter"], "1900-01-01T00:00:00Z");
}
