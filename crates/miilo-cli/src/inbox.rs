use crate::args::{InboxCommand, parse_target};
use chrono::{DateTime, Utc};
use miilo_adapter_sdk::{
    intake::{EXTENSION, IntakeDetails, action},
    *,
};
use miilo_client::MiiloClient;
use serde_json::Value;
use std::{io::Read, path::Path};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn invocation(command: &InboxCommand) -> Result<ActionInvocation> {
    let InboxCommand::Decide {
        target,
        action: decision,
        revision,
        request_id,
        confirmed_at,
        method,
        until,
    } = command
    else {
        return Err("Decision required".into());
    };
    let (provider, id) = parse_target(target)?;
    if revision.trim().is_empty() || request_id.trim().is_empty() || request_id.len() > 200 {
        return Err("Revision and bounded request ID required".into());
    }
    let action_id = match decision.as_str() {
        "accept" => action::ACCEPT,
        "defer" => action::DEFER,
        "dismiss" => action::DISMISS,
        _ => return Err("Unsupported action".into()),
    };
    let mut input = serde_json::Map::new();
    if action_id == action::DEFER {
        let until = until.as_ref().ok_or("Defer needs --until")?;
        DateTime::parse_from_rfc3339(until)?;
        input.insert("until".into(), until.clone().into());
    } else if until.is_some() {
        return Err("--until is only valid for defer".into());
    }
    Ok(ActionInvocation {
        protocol_version: "0.1".into(),
        request_id: request_id.clone(),
        provider_id: provider.into(),
        action_id: action_id.into(),
        subject: SubjectReference {
            kind: SubjectKind::Task,
            id: id.into(),
        },
        expected_revision: revision.clone(),
        confirmation: Confirmation {
            method: if method == "voice" {
                ConfirmationMethod::Voice
            } else {
                ConfirmationMethod::Manual
            },
            confirmed_at: DateTime::parse_from_rfc3339(confirmed_at)?.with_timezone(&Utc),
        },
        input,
        idempotency_key: Some(request_id.clone()),
    })
}
fn project(snapshot: ObservatorySnapshot, command: &InboxCommand) -> Result<Value> {
    let target = if let InboxCommand::Show { target } = command {
        Some(parse_target(target)?)
    } else {
        None
    };
    let mut items = Vec::new();
    for item in snapshot.work_items {
        if let Some((p, id)) = target {
            if item.provider_id != p || item.id != id {
                continue;
            }
        }
        let Some(value) = item.extensions.as_ref().and_then(|e| e.get(EXTENSION)) else {
            continue;
        };
        let details: IntakeDetails = serde_json::from_value(value.clone())?;
        details.validate()?;
        items.push(item);
    }
    if target.is_some() && items.is_empty() {
        return Err("Intake item not found".into());
    }
    Ok(serde_json::json!({"items":items,"cursor":snapshot.cursor}))
}
pub fn local(path: &Path, command: &InboxCommand) -> Result<Value> {
    let store = miilo_intake::storage::Files::new(path)?;
    if let InboxCommand::Dispatch { config } = command {
        let (config, executor) = miilo_intake::executor_http::load(config)?;
        return Ok(
            serde_json::json!({"updated":miilo_intake::execution::tick(&store,&executor,&config.workflows,Utc::now())?}),
        );
    }
    store.transaction(|state| {
        let now=Utc::now();
        // Retry receipts remain accessible before waking deferred items changes revisions.
        match command {
            InboxCommand::Import {file}=> {
                let mut bytes=Vec::new();std::fs::File::open(file).map_err(|_|"Cannot open submission")?.take(20001).read_to_end(&mut bytes).map_err(|_|"Cannot read submission")?;
                if bytes.len()>20000{return Err("Submission exceeds size bound".into());}
                let input=serde_json::from_slice(&bytes).map_err(|_|"Invalid submission JSON")?;
                Ok(serde_json::json!({"imported":miilo_intake::application::import(state,input,now)?}))
            }
            InboxCommand::Decide {..}=> {
                let request=invocation(command).map_err(|e|e.to_string())?;
                serde_json::to_value(miilo_intake::application::decide(state,request,now)?).map_err(|_|"Cannot encode action result".into())
            }
            _=>{miilo_intake::application::wake(state,now);project(miilo_intake::projection::snapshot(state,now),command).map_err(|e|e.to_string())}
        }
    }).map_err(Into::into)
}
pub async fn remote(client: &MiiloClient, command: InboxCommand) -> Result<Value> {
    match &command {
        InboxCommand::Dispatch { .. } => Err("Dispatch is a local runtime operation".into()),
        InboxCommand::Serve { .. } => Err("Serve is a local command".into()),
        InboxCommand::Import { .. } => Err(
            "Import requires --state; a remote adapter owns its own source import authority".into(),
        ),
        InboxCommand::Decide { .. } => {
            // Provider enforces current revision/permissions and replays matching receipts.
            // Do not prefetch a new revision here: an uncertain retry must retain its binding.
            let result = client.invoke(&invocation(&command)?).await?;
            if matches!(
                result.status,
                ActionStatus::Failed | ActionStatus::Rejected | ActionStatus::Conflict
            ) {
                return Err(format!(
                    "Action did not complete: {:?}; inspect provider state",
                    result.status
                )
                .into());
            }
            Ok(serde_json::to_value(result)?)
        }
        _ => project(client.snapshot().await?, &command),
    }
}
