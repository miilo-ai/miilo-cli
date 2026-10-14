use crate::{ClientError, StreamEvent};
use futures_util::{Stream, StreamExt, stream};
use miilo_adapter_sdk::ObservatoryEvent;
use serde::Deserialize;
use std::{collections::VecDeque, pin::Pin};

const MAX_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Default)]
struct Decoder {
    buffer: Vec<u8>,
}

impl Decoder {
    fn push(&mut self, bytes: &[u8]) -> Result<Vec<StreamEvent>, ClientError> {
        self.buffer.extend_from_slice(bytes);
        let mut events = Vec::new();
        while let Some((end, delimiter)) = frame_end(&self.buffer) {
            let frame: Vec<u8> = self.buffer.drain(..end).collect();
            self.buffer.drain(..delimiter);
            if let Some(event) = decode_frame(&frame)? {
                events.push(event);
            }
        }
        if self.buffer.len() > MAX_FRAME_BYTES {
            return Err(ClientError::StreamFrameTooLarge);
        }
        Ok(events)
    }

    fn finish(self) -> Result<(), ClientError> {
        if self.buffer.iter().all(u8::is_ascii_whitespace) {
            Ok(())
        } else {
            Err(ClientError::IncompleteStreamFrame)
        }
    }
}

fn frame_end(buffer: &[u8]) -> Option<(usize, usize)> {
    let crlf = buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|end| (end, 4));
    let lf = buffer
        .windows(2)
        .position(|window| window == b"\n\n")
        .map(|end| (end, 2));
    match (crlf, lf) {
        (Some(left), Some(right)) => Some(if left.0 <= right.0 { left } else { right }),
        (Some(found), None) | (None, Some(found)) => Some(found),
        (None, None) => None,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamError {
    reason: String,
    message: String,
}

fn decode_frame(frame: &[u8]) -> Result<Option<StreamEvent>, ClientError> {
    let text = String::from_utf8_lossy(frame);
    let mut id = None;
    let mut kind = "message";
    let mut data = Vec::new();
    for line in text.lines() {
        if line.starts_with(':') {
            continue;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match field {
            "id" => id = Some(value.to_owned()),
            "event" => kind = value,
            "data" => data.push(value),
            _ => {}
        }
    }
    if data.is_empty() {
        return Ok(None);
    }
    let data = data.join("\n");
    match kind {
        "miilo.event" => Ok(Some(StreamEvent::Event {
            stream_cursor: id.ok_or(ClientError::MissingStreamCursor)?,
            event: Box::new(serde_json::from_str::<ObservatoryEvent>(&data)?),
        })),
        "miilo.error" => {
            let error: StreamError = serde_json::from_str(&data)?;
            Ok(Some(StreamEvent::Error {
                reason: error.reason,
                message: error.message,
            }))
        }
        _ => Ok(None),
    }
}

struct State {
    upstream: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    decoder: Decoder,
    queued: VecDeque<StreamEvent>,
    exhausted: bool,
}

pub(crate) fn decode(
    upstream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ClientError>> + Send>> {
    let state = State {
        upstream: Box::pin(upstream),
        decoder: Decoder::default(),
        queued: VecDeque::new(),
        exhausted: false,
    };
    Box::pin(stream::unfold(state, |mut state| async move {
        loop {
            if let Some(event) = state.queued.pop_front() {
                return Some((Ok(event), state));
            }
            if state.exhausted {
                return None;
            }
            match state.upstream.next().await {
                Some(Ok(bytes)) => match state.decoder.push(&bytes) {
                    Ok(events) => state.queued.extend(events),
                    Err(error) => {
                        state.exhausted = true;
                        return Some((Err(error), state));
                    }
                },
                Some(Err(error)) => {
                    state.exhausted = true;
                    return Some((Err(error.into()), state));
                }
                None => {
                    state.exhausted = true;
                    if let Err(error) = std::mem::take(&mut state.decoder).finish() {
                        return Some((Err(error), state));
                    }
                }
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_chunked_event_and_ignores_heartbeat() {
        let mut decoder = Decoder::default();
        assert!(decoder.push(b": heartbeat\n\n").unwrap().is_empty());
        assert!(
            decoder
                .push(b"id: aggregate:abc\nevent: miilo.event\ndata: {\"protocolVersion\":\"0.1\",\"eventId\":\"e")
                .unwrap()
                .is_empty()
        );
        let events = decoder
            .push(b"1\",\"cursor\":\"p:1\",\"providerId\":\"p\",\"occurredAt\":\"2026-10-12T00:00:00Z\",\"kind\":\"updated\",\"subject\":{\"kind\":\"task\",\"id\":\"w\"},\"summary\":\"changed\",\"payload\":{}}\n\n")
            .unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::Event {
                stream_cursor,
                event,
            } => {
                assert_eq!(stream_cursor, "aggregate:abc");
                assert_eq!(event.event_id, "e1");
            }
            StreamEvent::Error { .. } => panic!("expected an event"),
        }
    }

    #[test]
    fn decodes_multiple_frames_from_one_large_chunk() {
        let mut decoder = Decoder::default();
        let frame = b": one\n\n: two\n\nevent: miilo.error\ndata: {\"reason\":\"retry\",\ndata: \"message\":\"refresh\"}\n\n";
        let events = decoder.push(frame).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            StreamEvent::Error { reason, message }
                if reason == "retry" && message == "refresh"
        ));
    }
}
