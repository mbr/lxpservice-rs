use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// Some Enums for lxpapi
#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum ColorPrint {
    Color,
    BlackAndWhite,
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum Mode {
    Simplex,
    Duplex,
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum Ship {
    National,
    International,
}

// Substructures used in request and response structs
#[allow(dead_code)]
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SubAuth {
    pub id: String,
    pub user: String,
    pub status: String,
}

#[allow(dead_code)]
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SubBalance {
    pub value: String,
    pub currency: String,
}

#[allow(dead_code)]
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SubInvoice {
    pub iid: String,
    pub invoicedate: String,
    pub pdf_data: Option<String>,
    pub bin_pdf_data: Option<Vec<u8>>,
    pub sum: String,
    pub vat: String,
}

#[allow(dead_code)]
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SubJobArgs {
    pub jid: String,
    pub address: String,
    pub parent: Option<String>,
    pub status: String,
    pub mode: String,
    pub color: String,
    pub cover: String,
    pub shipping: String,
    pub pages: String,
    pub cost: String,
    pub cost_vat: String,
    pub date: String,
    pub dispatchdate: Option<String>,
    pub sentdate: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SubLetterData {
    pub base64_file: String,
    pub base64_checksum: String,
    pub address: String,
    pub specification: SubSpecification,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SubNameAndKey {
    pub username: String,
    pub apikey: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SubSpecification {
    pub color: i32,
    pub mode: String,
    pub ship: String,
}

// Request structs
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct RequestLetter {
    pub auth: SubNameAndKey,
    pub letter: SubLetterData,
}

// Response struct
#[allow(dead_code)]
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Response {
    pub auth: Option<SubAuth>,
    pub balance: Option<SubBalance>,
    pub invoice: Option<SubInvoice>,
    pub invoices: Option<HashMap<String, SubInvoice>>,
    pub jobs: Option<HashMap<String, SubJobArgs>>,
    pub status: i32,
    pub message: String,
}
