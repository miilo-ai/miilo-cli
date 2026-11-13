use serde_json::{Value, json};
pub fn snapshot() -> Value {
    json!({"protocolVersion":"0.1","snapshotId":"snapshot-private","observedAt":"2026-11-12T00:00:00Z","cursor":"boot:1","providers":[],"actors":[],"attempts":[],"artifacts":[],"evidence":[],"attention":[],"workItems":[{
        "id":"private-work","providerId":"private-provider","kind":"task","title":"PRIVATE WORK TITLE","summary":"PRIVATE SUMMARY","lifecycle":"active","phase":"implementing","attention":"none","health":"healthy","providerState":"running","revision":"1","dependencyIds":[],"actions":[],"updatedAt":"2026-11-12T00:00:00Z"
    }]})
}
pub fn explanation() -> Value {
    json!({"protocolVersion":"0.1","explanationId":"private-explanation","providerId":"private-provider","workItemId":"private-work","title":"PRIVATE WORK TITLE","revision":"1","generatedAt":"2026-11-12T00:00:00Z","observedAt":"2026-11-12T00:00:00Z","condition":"working","cause":"PRIVATE CAUSE","impact":"PRIVATE IMPACT","recommendation":"PRIVATE RECOMMENDATION","spokenSummary":"PRIVATE SPOKEN","confidence":"supported","evidence":[],"changes":[],"actions":[],"uncertainty":[],"sourceIds":[]})
}
