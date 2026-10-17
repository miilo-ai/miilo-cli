use chrono::{DateTime, Utc};
use serde_json::{Value, json};

pub fn wildcard(pattern: &str, name: &str) -> bool {
    // Bounded dynamic programming: '*' and '?' only, no regex or shell evaluation.
    let chars: Vec<_> = name.chars().collect();
    let mut row = vec![false; chars.len() + 1];
    row[0] = true;
    for p in pattern.chars() {
        let mut next = vec![false; chars.len() + 1];
        if p == '*' {
            next[0] = row[0];
        }
        for j in 1..=chars.len() {
            next[j] = if p == '*' {
                row[j] || next[j - 1]
            } else {
                row[j - 1] && (p == '?' || p == chars[j - 1])
            };
        }
        row = next;
    }
    row[chars.len()]
}
pub fn future_matches(rule: &Value, session: &Value, in_scope: bool, now: DateTime<Utc>) -> bool {
    if rule["future"] != true || !in_scope {
        return false;
    }
    let Some(cutoff) = rule["enrolledAfter"]
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
    else {
        return false;
    };
    let created = session["createdAt"]
        .as_i64()
        .and_then(|n| chrono::DateTime::from_timestamp(n, 0))
        .or_else(|| {
            session["createdAt"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|v| v.with_timezone(&chrono::Utc))
        });
    created.is_some_and(|at| {
        at > cutoff.with_timezone(&chrono::Utc) && at <= now + chrono::Duration::seconds(5)
    })
}
pub fn bind_consent(rules: &mut [Value], previous: &Value, now: DateTime<Utc>) {
    for rule in rules {
        let existing = previous.as_array().into_iter().flatten().find(|old| {
            old["name"] == rule["name"]
                && old["folder"] == rule["folder"]
                && old["pattern"] == rule["pattern"]
                && old["kind"] == rule["kind"]
                && old["future"] == true
        });
        rule["enrolledAfter"] = if rule["future"] == true {
            existing
                .and_then(|old| old["enrolledAfter"].as_str())
                .map(|s| json!(s))
                .unwrap_or_else(|| json!(now.to_rfc3339()))
        } else {
            Value::Null
        };
    }
}
