use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Miilo endpoint must not contain credentials, a query, or a fragment")]
    UnsafeEndpoint,
    #[error("invalid Miilo endpoint: {0}")]
    InvalidEndpoint(#[from] url::ParseError),
    #[error("Miilo endpoint must use HTTP or HTTPS, not {0}")]
    UnsupportedEndpointScheme(String),
    #[error("Miilo transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Miilo returned HTTP {status}: {message}")]
    Http { status: u16, message: String },
    #[error("Miilo returned an invalid payload: {0}")]
    InvalidPayload(#[from] serde_json::Error),
    #[error("Miilo event stream frame exceeded 1 MiB")]
    StreamFrameTooLarge,
    #[error("Miilo event stream ended with an incomplete frame")]
    IncompleteStreamFrame,
    #[error("Miilo event stream frame is missing its resumable ID")]
    MissingStreamCursor,
    #[error("Miilo returned {0} instead of text/event-stream")]
    UnexpectedContentType(String),
}
