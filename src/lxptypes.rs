//! Models LetterXpress requests and responses independently of transport.

use std::num::NonZeroU64;

use base64::{Engine, engine::general_purpose::STANDARD};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Limits raw PDFs to leave room for Base64 and JSON in a 50 MB request.
pub const MAX_PDF_BYTES: usize = 35_000_000;

/// Selects shopping-cart uploads or immediate processing.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ApiMode {
    /// Holds uploads in the account's shopping cart without printing.
    #[default]
    Test,
    /// Immediately enters the paid processing workflow.
    Live,
}

/// Selects printing colour.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ValueEnum)]
pub enum Color {
    /// Prints with black ink.
    #[default]
    #[serde(rename = "1")]
    Bw,
    /// Prints in colour.
    #[serde(rename = "4")]
    Color,
}

/// Selects the printed sides of a sheet.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Sides {
    /// Prints one side.
    #[default]
    Simplex,
    /// Prints both sides.
    Duplex,
}

/// Selects the delivery region.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Shipping {
    /// Delivers within Germany.
    #[default]
    National,
    /// Delivers outside Germany.
    International,
}

/// Selects a job-list filter.
#[derive(Clone, Copy, Debug, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum JobFilter {
    /// Selects jobs waiting for processing.
    Queue,
    /// Selects jobs on hold.
    Hold,
    /// Selects completed processing, not confirmed delivery.
    Done,
    /// Selects canceled jobs.
    Canceled,
    /// Selects shopping-cart jobs.
    Draft,
}

/// Carries authentication for a single request.
#[derive(Serialize)]
pub struct Auth<'a> {
    /// Identifies the account.
    pub username: &'a str,
    /// Authenticates the account.
    pub apikey: &'a str,
    /// Determines whether uploads may enter production.
    pub mode: ApiMode,
}

/// Carries authenticated request data.
#[derive(Serialize)]
pub struct Request<'a, T> {
    /// Authenticates this request.
    pub auth: Auth<'a>,
    /// Specifies an optional document or quote.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub letter: Option<T>,
}

/// Specifies print and delivery options.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct Specification {
    /// Selects ink usage.
    pub color: Color,
    /// Selects sheet sides.
    pub mode: Sides,
    /// Selects delivery region.
    pub shipping: Shipping,
    /// Supplies page count for quotes, not uploads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<u32>,
}

/// Carries an encoded PDF and its print options.
#[derive(Serialize)]
pub struct Letter {
    /// Encodes the original PDF bytes.
    pub base64_file: String,
    /// Checksums the Base64 string rather than the original bytes.
    pub base64_file_checksum: String,
    /// Configures printing and delivery.
    pub specification: Specification,
    /// Identifies the local document in the account.
    pub filename_original: String,
    /// Correlates submissions without providing idempotency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
}

impl Letter {
    /// Encodes a bounded PDF payload without performing I/O.
    pub fn from_pdf(
        pdf: &[u8],
        filename: String,
        specification: Specification,
        notice: Option<String>,
    ) -> Result<Self, DocumentError> {
        if pdf.len() > MAX_PDF_BYTES {
            return Err(DocumentError::TooLarge);
        }
        if !pdf.starts_with(b"%PDF-") {
            return Err(DocumentError::InvalidHeader);
        }
        if notice
            .as_ref()
            .is_some_and(|value| value.chars().count() > 255)
        {
            return Err(DocumentError::NoticeTooLong);
        }
        let base64_file = STANDARD.encode(pdf);
        let base64_file_checksum = format!("{:x}", md5::compute(&base64_file));
        Ok(Self {
            base64_file,
            base64_file_checksum,
            specification,
            filename_original: filename,
            notice,
        })
    }
}

/// Describes local document validation failures.
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    /// Rejects oversized input before encoding.
    #[error("PDF exceeds the 35 MB raw-document limit")]
    TooLarge,
    /// Rejects files that are not PDFs.
    #[error("document does not start with a PDF header")]
    InvalidHeader,
    /// Rejects notices outside the API limit.
    #[error("notice exceeds 255 characters")]
    NoticeTooLong,
}

/// Requests a price without a document.
#[derive(Serialize)]
pub struct Quote {
    /// Specifies print options and page count.
    pub specification: Specification,
}

/// Wraps a service response.
#[derive(Deserialize)]
pub struct Response<T> {
    /// Reports the service-level result code.
    pub status: u16,
    /// Carries endpoint-specific results.
    pub data: Option<T>,
}

/// Reports available account credit.
#[derive(Debug, Deserialize, Serialize)]
pub struct Balance {
    /// Gives the available credit.
    pub balance: f64,
    /// Identifies the currency.
    pub currency: String,
}

/// Reports an estimated price.
#[derive(Debug, Deserialize, Serialize)]
pub struct Price {
    /// Gives the quoted service price.
    pub price: f64,
}

/// Represents an accepted print job.
#[derive(Debug, Deserialize, Serialize)]
pub struct PrintJob {
    /// Identifies the parent print job.
    pub id: NonZeroU64,
    /// Reports processing state, not proof of delivery.
    pub status: String,
    /// Identifies the uploaded file when returned.
    pub filename_original: Option<String>,
    /// Carries the caller's correlation reference.
    pub notice: Option<String>,
    /// Reports individual letters in the job.
    pub items: Vec<PrintItem>,
}

/// Reports an individual letter within a print job.
#[derive(Debug, Deserialize, Serialize)]
pub struct PrintItem {
    /// Gives the recognized destination address.
    pub address: String,
    /// Gives the printed page count.
    pub pages: u32,
    /// Gives the net amount.
    pub amount: f64,
    /// Gives the tax amount.
    pub vat: f64,
    /// Reports item processing state.
    pub status: String,
    /// Identifies registered-mail tracking when available.
    pub tracking_code: Option<String>,
    /// Reports carrier tracking when available.
    pub tracking_status: Option<String>,
}

/// Reports a page of print jobs.
#[derive(Debug, Deserialize, Serialize)]
pub struct Jobs {
    /// Contains the page's print jobs.
    pub printjobs: Vec<PrintJob>,
    /// Describes remaining pages.
    pub pagination: Pagination,
}

/// Describes numeric pagination without trusting server-supplied URLs.
#[derive(Debug, Deserialize, Serialize)]
pub struct Pagination {
    /// Identifies the returned page.
    pub current_page: u32,
    /// Identifies the last available page.
    pub last_page: u32,
}

/// Reports a page of invoices.
#[derive(Debug, Deserialize, Serialize)]
pub struct Invoices {
    /// Contains the page's invoices.
    pub invoices: Vec<Invoice>,
    /// Describes remaining pages.
    pub pagination: Pagination,
}

/// Represents an invoice and optional PDF contents.
#[derive(Debug, Deserialize, Serialize)]
pub struct Invoice {
    /// Identifies the invoice.
    pub id: NonZeroU64,
    /// Gives the net amount.
    pub amount: f64,
    /// Gives the tax amount.
    pub vat: f64,
    /// Gives the issue date.
    pub invoice_date: String,
    /// Carries Base64 PDF data for individual invoice requests.
    #[serde(skip_serializing)]
    pub base64_data: Option<String>,
}

#[cfg(test)]
mod tests {
    use base64::{Engine, engine::general_purpose::STANDARD};

    use super::{ApiMode, Auth, DocumentError, Letter, Request, Specification};

    /// Verifies the wire checksum, string options and explicit test mode.
    #[test]
    fn encoded_letter_contract() {
        let pdf = b"%PDF-1.7\nfixture";
        let letter = Letter::from_pdf(pdf, "test.pdf".into(), Specification::default(), None)
            .expect("valid PDF header");
        assert_eq!(
            STANDARD.decode(&letter.base64_file).expect("valid Base64"),
            pdf
        );
        assert_eq!(
            letter.base64_file_checksum,
            format!("{:x}", md5::compute(STANDARD.encode(pdf)))
        );
        assert_ne!(
            letter.base64_file_checksum,
            format!("{:x}", md5::compute(pdf))
        );
        let value = serde_json::to_value(Request {
            auth: Auth {
                username: "user",
                apikey: "dummy",
                mode: ApiMode::Test,
            },
            letter: Some(letter),
        })
        .expect("serializable request");
        assert_eq!(value["auth"]["mode"], "test");
        assert_eq!(value["letter"]["specification"]["color"], "1");
        assert_eq!(value["letter"]["specification"]["shipping"], "national");
        assert!(value["letter"]["specification"].get("pages").is_none());
        assert!(value["letter"].get("base64_checksum").is_none());
    }

    /// Rejects invalid input before transport.
    #[test]
    fn rejects_invalid_document() {
        assert!(matches!(
            Letter::from_pdf(
                b"not PDF",
                "test.pdf".into(),
                Specification::default(),
                None
            ),
            Err(DocumentError::InvalidHeader)
        ));
        assert!(matches!(
            Letter::from_pdf(
                b"%PDF-",
                "test.pdf".into(),
                Specification::default(),
                Some("x".repeat(256))
            ),
            Err(DocumentError::NoticeTooLong)
        ));
    }
}
