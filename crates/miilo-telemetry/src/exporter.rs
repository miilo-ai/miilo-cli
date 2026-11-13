use crate::Config;
use opentelemetry_proto::{
    tonic::collector::trace::v1::{ExportTraceServiceRequest, ExportTraceServiceResponse},
    transform::{
        common::tonic::ResourceAttributesWithSchema,
        trace::tonic::group_spans_by_resource_and_scope,
    },
};
use opentelemetry_sdk::{
    Resource,
    error::{OTelSdkError, OTelSdkResult},
    trace::{SpanData, SpanExporter},
};
use prost::Message;
use std::{fmt, io::Read, time::Duration};

pub(crate) struct Exporter {
    client: reqwest::blocking::Client,
    config: Config,
    resource: ResourceAttributesWithSchema,
    counters: std::sync::Arc<crate::stats::Counters>,
}
impl fmt::Debug for Exporter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MiiloOtlpExporter")
    }
}
impl Exporter {
    pub fn new(
        config: Config,
        counters: std::sync::Arc<crate::stats::Counters>,
    ) -> Result<Self, &'static str> {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_millis(250))
            .timeout(Duration::from_millis(500))
            .build()
            .map_err(|_| "telemetry transport initialization failed")?;
        Ok(Self {
            client,
            config,
            resource: ResourceAttributesWithSchema::default(),
            counters,
        })
    }
    fn send(&self, body: Vec<u8>) -> OTelSdkResult {
        for attempt in 0..2 {
            let mut request = self
                .client
                .post(self.config.endpoint.clone())
                .header("content-type", "application/x-protobuf")
                .body(body.clone());
            if let Some(token) = &self.config.token {
                request = request.bearer_auth(token);
            }
            let response = request.send();
            match response {
                Ok(mut response) if response.status().is_success() => {
                    let mut bytes = Vec::new();
                    response
                        .by_ref()
                        .take(65_537)
                        .read_to_end(&mut bytes)
                        .map_err(|_| failed())?;
                    if bytes.len() > 65_536 {
                        return Err(failed());
                    }
                    let result = ExportTraceServiceResponse::decode(bytes.as_slice())
                        .map_err(|_| failed())?;
                    return if result.partial_success.is_some_and(|p| p.rejected_spans > 0) {
                        Err(failed())
                    } else {
                        Ok(())
                    };
                }
                Ok(response) if !matches!(response.status().as_u16(), 429 | 502 | 503 | 504) => {
                    return Err(failed());
                }
                _ if attempt == 0 => std::thread::sleep(Duration::from_millis(25)),
                _ => return Err(failed()),
            }
        }
        Err(failed())
    }
}
fn failed() -> OTelSdkError {
    OTelSdkError::InternalFailure("telemetry delivery failed".into())
}
impl SpanExporter for Exporter {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        let count = batch.len() as u64;
        let body = ExportTraceServiceRequest {
            resource_spans: group_spans_by_resource_and_scope(batch, &self.resource),
        }
        .encode_to_vec();
        let result = self.send(body);
        if result.is_ok() {
            self.counters
                .delivered
                .fetch_add(count, std::sync::atomic::Ordering::Relaxed);
        } else {
            self.counters
                .failed_batches
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        result
    }
    fn set_resource(&mut self, resource: &Resource) {
        self.resource = resource.into();
    }
}
