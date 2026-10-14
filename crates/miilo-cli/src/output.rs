use miilo_adapter_sdk::{ObservatorySnapshot, WorkItem};
use miilo_client::{ConnectionsResponse, ExplanationBundle, HealthResponse, StreamEvent};
use serde::Serialize;

pub fn json(value: &impl Serialize) -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn json_line(value: &impl Serialize) -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string(value)?);
    Ok(())
}

pub fn doctor(health: &HealthResponse, snapshot: &ObservatorySnapshot) {
    println!("Miilo API: ready (protocol {})", health.protocol_version);
    println!("Providers: {}", snapshot.providers.len());
    println!("Work items: {}", snapshot.work_items.len());
    println!("Attention items: {}", snapshot.attention.len());
    println!("Identity: {}", configured(health.identity));
    println!("Storage: {}", configured(health.storage));
}

pub fn connections(response: &ConnectionsResponse) {
    if response.connections.is_empty() {
        println!("No connections configured.");
        return;
    }
    for connection in &response.connections {
        let state = if connection.enabled {
            "enabled"
        } else {
            "disabled"
        };
        println!(
            "{}\t{}\t{}\t{}",
            clean(&connection.id),
            clean(&connection.name),
            clean(&connection.adapter),
            state
        );
    }
}

pub fn work(items: &[WorkItem]) {
    if items.is_empty() {
        println!("No work items found.");
        return;
    }
    for item in items {
        println!(
            "{}/{}\t{:?}\t{:?}\t{}",
            clean(&item.provider_id),
            clean(&item.id),
            item.lifecycle,
            item.phase,
            clean(&item.title)
        );
    }
}

pub fn work_item(item: &WorkItem) {
    println!("{}", clean(&item.title));
    println!("ID: {}/{}", clean(&item.provider_id), clean(&item.id));
    println!("State: {:?} / {:?}", item.lifecycle, item.phase);
    println!("Health: {:?}", item.health);
    println!("Attention: {:?}", item.attention);
    println!("Revision: {}", clean(&item.revision));
    println!("{}", clean(&item.summary));
}

pub fn explanation(bundle: &ExplanationBundle) {
    println!("{}", clean(&bundle.title));
    println!("{}", clean(&bundle.spoken_summary));
    println!("Confidence: {:?}", bundle.confidence);
    if !bundle.recommendation.is_empty() {
        println!("Recommendation: {}", clean(&bundle.recommendation));
    }
}

pub fn stream_event(event: &StreamEvent) {
    match event {
        StreamEvent::Event { event, .. } => println!(
            "{}\t{}\t{:?}\t{}",
            event.occurred_at,
            clean(&event.provider_id),
            event.kind,
            clean(&event.summary)
        ),
        StreamEvent::Error { reason, message } => eprintln!("{reason}: {message}"),
    }
}

fn configured(value: bool) -> &'static str {
    if value {
        "configured"
    } else {
        "development mode"
    }
}

fn clean(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::clean;

    #[test]
    fn removes_terminal_control_sequences() {
        assert_eq!(clean("safe\u{1b}[31m"), "safe [31m");
    }
}
