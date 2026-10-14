use miilo_adapter_sdk::ObservatoryEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub ok: bool,
    pub service: String,
    pub protocol_version: String,
    pub providers: usize,
    pub identity: bool,
    pub storage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionSummary {
    pub id: String,
    pub name: String,
    pub adapter: String,
    pub enabled: bool,
    pub credential_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionsResponse {
    pub connections: Vec<ConnectionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    Event {
        stream_cursor: String,
        event: Box<ObservatoryEvent>,
    },
    Error {
        reason: String,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplanationDetail {
    Concise,
    #[default]
    Normal,
    Detailed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplanationLead {
    #[default]
    Cause,
    Impact,
    Recommendation,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplanationPreferences {
    #[serde(default)]
    pub detail: ExplanationDetail,
    #[serde(default)]
    pub lead: ExplanationLead,
    #[serde(default)]
    pub include_technical_details: bool,
    #[serde(default)]
    pub speak_sensitive_values: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplanationRequest {
    pub provider_id: String,
    pub work_item_id: String,
    pub attempt_id: Option<String>,
    pub actor_id: Option<String>,
    pub expected_revision: Option<String>,
    pub question: Option<String>,
    #[serde(default)]
    pub preferences: ExplanationPreferences,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplanationConfidence {
    Supported,
    Inferred,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplanationEvidenceRef {
    pub id: String,
    pub kind: String,
    pub outcome: String,
    pub summary: String,
    pub recorded_at: String,
    pub artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplanationChange {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub digest: Option<String>,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplanationAction {
    pub id: String,
    pub label: String,
    pub risk: String,
    pub confirmation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplanationBundle {
    pub protocol_version: String,
    pub explanation_id: String,
    pub provider_id: String,
    pub work_item_id: String,
    pub title: String,
    pub revision: String,
    pub generated_at: String,
    pub observed_at: String,
    pub condition: String,
    pub cause: String,
    pub impact: String,
    pub recommendation: String,
    pub spoken_summary: String,
    pub confidence: ExplanationConfidence,
    pub evidence: Vec<ExplanationEvidenceRef>,
    pub changes: Vec<ExplanationChange>,
    pub actions: Vec<ExplanationAction>,
    pub uncertainty: Vec<String>,
    pub source_ids: Vec<String>,
}
