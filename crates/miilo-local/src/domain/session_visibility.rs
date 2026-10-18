use chrono::{DateTime, Utc};

/// Selected transcript sessions remain visible for twelve hours after activity.
/// Missing, malformed and future timestamps cannot establish recent activity.
pub fn recently_observed(timestamp: &str, now: DateTime<Utc>) -> bool {
    DateTime::parse_from_rfc3339(timestamp).is_ok_and(|at| {
        let age = now.signed_duration_since(at.with_timezone(&Utc));
        age >= chrono::Duration::zero() && age <= chrono::Duration::hours(12)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn twelve_hour_boundary_uses_activity_time() {
        let now: DateTime<Utc> = "2026-10-17T12:00:00Z".parse().unwrap();
        for (timestamp, expected) in [
            ("2026-10-17T12:00:00Z", true),
            ("2026-10-17T00:00:00Z", true),
            ("2026-10-16T23:59:59Z", false),
            ("2026-10-17T12:00:01Z", false),
            ("invalid", false),
            ("", false),
        ] {
            assert_eq!(recently_observed(timestamp, now), expected, "{timestamp}");
        }
    }
}
