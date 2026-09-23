//! Transports typed LetterXpress requests without logging sensitive payloads.

use std::{num::NonZeroU64, time::Duration};

use reqwest::{Client, Method, StatusCode};
use serde::{de::DeserializeOwned, Serialize};

use crate::lxptypes::{
    ApiMode, Auth, Balance, Invoice, Invoices, JobFilter, Jobs, Letter, Price, PrintJob, Quote,
    Request, Response, Specification,
};

/// Limits server responses, including Base64 invoice data.
const MAX_RESPONSE_BYTES: usize = 50_000_000;

/// Owns credentials and a reusable HTTP connection pool.
pub struct LxpApi {
    /// Locates the API version root.
    base_url: String,
    /// Identifies the account.
    username: String,
    /// Authenticates requests.
    apikey: String,
    /// Determines upload processing semantics.
    mode: ApiMode,
    /// Pools HTTPS connections without retries or redirects.
    client: Client,
}

/// Describes failures without including response bodies or credentials.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reports a network or TLS failure.
    #[error("HTTP transport failed")]
    Transport {
        /// Preserves the underlying transport error.
        #[source]
        source: reqwest::Error,
    },
    /// Reports a rejected HTTP request.
    #[error("HTTP request rejected with status {status}")]
    Http {
        /// Gives the HTTP status code.
        status: StatusCode,
    },
    /// Reports a service-level error even when HTTP succeeded.
    #[error("API request rejected with status {status}")]
    Api {
        /// Gives the service status code.
        status: u16,
    },
    /// Reports malformed or unexpected response data.
    #[error("invalid API response")]
    Json {
        /// Preserves decoding context without returning the payload.
        #[source]
        source: serde_json::Error,
    },
    /// Rejects successful responses lacking endpoint data.
    #[error("API response is missing data")]
    MissingData,
    /// Bounds memory used for server responses.
    #[error("API response exceeds the size limit")]
    ResponseTooLarge,
    /// Requires a separate opt-in before paid submissions.
    #[error("live submission requires --yes; test mode does not print")]
    LiveConfirmationRequired,
    /// Warns that a failed upload may already have been accepted.
    #[error("submission not confirmed; inspect recent jobs before retrying")]
    Unconfirmed {
        /// Preserves the original failure.
        #[source]
        source: Box<Error>,
    },
}

impl LxpApi {
    /// Creates a client for the production service with explicit processing mode.
    pub fn new(username: String, apikey: String, mode: ApiMode) -> Result<Self, Error> {
        let client = client(true)?;
        Ok(Self {
            base_url: "https://api.letterxpress.de/v3".into(),
            username,
            apikey,
            mode,
            client,
        })
    }

    /// Retrieves the available account balance.
    pub async fn balance(&self) -> Result<Balance, Error> {
        self.get("balance").await
    }

    /// Quotes a document without uploading its contents.
    pub async fn price(&self, specification: Specification) -> Result<Price, Error> {
        self.request(Method::GET, "price", Some(Quote { specification }))
            .await?
            .ok_or(Error::MissingData)
    }

    /// Retrieves a page of jobs without following server-supplied URLs.
    pub async fn jobs(&self, filter: Option<JobFilter>, page: u32) -> Result<Jobs, Error> {
        let mut path = format!("printjobs?page={page}");
        if let Some(filter) = filter {
            let value = serde_json::to_value(filter).map_err(|source| Error::Json { source })?;
            if let Some(filter) = value.as_str() {
                path.push_str("&filter=");
                path.push_str(filter);
            }
        }
        self.get(&path).await
    }

    /// Retrieves processing and tracking information for a job.
    pub async fn job(&self, id: NonZeroU64) -> Result<PrintJob, Error> {
        self.get(&format!("printjobs/{id}")).await
    }

    /// Cancels a job subject to the provider's cancellation window.
    pub async fn cancel(&self, id: NonZeroU64) -> Result<(), Error> {
        self.request::<(), ()>(Method::DELETE, &format!("printjobs/{id}"), None)
            .await?;
        Ok(())
    }

    /// Submits a document once, guarding the effective mode rather than its source.
    pub async fn send(&self, letter: Letter, confirmed: bool) -> Result<PrintJob, Error> {
        if self.mode == ApiMode::Live && !confirmed {
            return Err(Error::LiveConfirmationRequired);
        }
        self.request(Method::POST, "printjobs", Some(letter))
            .await
            .and_then(|job| job.ok_or(Error::MissingData))
            .map_err(|source| Error::Unconfirmed {
                source: Box::new(source),
            })
    }

    /// Retrieves a page of invoices.
    pub async fn invoices(&self, page: u32) -> Result<Invoices, Error> {
        self.get(&format!("invoices?page={page}")).await
    }

    /// Retrieves a single invoice including its encoded PDF.
    pub async fn invoice(&self, id: NonZeroU64) -> Result<Invoice, Error> {
        self.get(&format!("invoices/{id}")).await
    }

    /// Requests endpoint data with an authenticated GET body.
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        self.request::<(), T>(Method::GET, path, None)
            .await?
            .ok_or(Error::MissingData)
    }

    /// Performs a single bounded request without logging request or response bodies.
    async fn request<L: Serialize, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        letter: Option<L>,
    ) -> Result<Option<T>, Error> {
        let body = Request {
            auth: Auth {
                username: &self.username,
                apikey: &self.apikey,
                mode: self.mode,
            },
            letter,
        };
        let mut response = self
            .client
            .request(method, format!("{}/{path}", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|source| Error::Transport { source })?;
        if !response.status().is_success() {
            return Err(Error::Http {
                status: response.status(),
            });
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|source| Error::Transport { source })?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(Error::ResponseTooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        decode(&bytes)
    }
}

/// Configures transport policy; production callers require HTTPS.
fn client(https_only: bool) -> Result<Client, Error> {
    Client::builder()
        .https_only(https_only)
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|source| Error::Transport { source })
}

#[cfg(test)]
mod transport_tests;

/// Validates a service envelope independently of HTTP transport.
fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<Option<T>, Error> {
    let response: Response<T> =
        serde_json::from_slice(bytes).map_err(|source| Error::Json { source })?;
    if !(200..300).contains(&response.status) {
        return Err(Error::Api {
            status: response.status,
        });
    }
    Ok(response.data)
}

#[cfg(test)]
mod tests {
    use super::{decode, Error, LxpApi};
    use crate::lxptypes::{ApiMode, Balance, Letter, Specification};

    /// Handles success, service errors and body-less cancellation responses.
    #[test]
    fn response_envelopes() {
        let balance = decode::<Balance>(br#"{"status":200,"data":{"balance":5,"currency":"EUR"}}"#)
            .expect("valid envelope")
            .expect("balance data");
        assert_eq!(balance.balance, 5.0);
        assert!(decode::<()>(br#"{"status":200,"message":"deleted"}"#)
            .expect("valid cancellation")
            .is_none());
        assert!(matches!(
            decode::<Balance>(br#"{"status":400,"message":"bad request"}"#),
            Err(Error::Api { status: 400 })
        ));
        assert!(matches!(
            decode::<Balance>(b"not json"),
            Err(Error::Json { .. })
        ));
    }

    /// Blocks live uploads before any transport activity.
    #[tokio::test]
    async fn live_requires_confirmation() {
        let api = LxpApi::new("dummy".into(), "dummy".into(), ApiMode::Live).expect("valid client");
        let letter = Letter::from_pdf(
            b"%PDF-1.7",
            "test.pdf".into(),
            Specification::default(),
            None,
        )
        .expect("valid header");
        assert!(matches!(
            api.send(letter, false).await,
            Err(Error::LiveConfirmationRequired)
        ));
    }
}
