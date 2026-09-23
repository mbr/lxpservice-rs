//! Executes explicit client operations and bounded document submissions.

use std::{collections::HashSet, path::Path};

use base64::{engine::general_purpose::STANDARD, Engine};
use notify::{
    event::{AccessKind, AccessMode, ModifyKind, RenameMode},
    EventKind, RecursiveMode, Watcher,
};
use serde::Serialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc::unbounded_channel,
};

use crate::{
    clidef::{Command, InvoiceCommand, Send},
    lxpapi::{self, LxpApi},
    lxptypes::{DocumentError, Letter, PrintJob, Specification, MAX_PDF_BYTES},
};

/// Describes command failures without relying on logging side effects.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reports local profile failure.
    #[error("{0}")]
    Config(#[source] crate::lxpconfig::Error),
    /// Reports a service failure.
    #[error("{0}")]
    Api(#[source] lxpapi::Error),
    /// Reports invalid document input.
    #[error("{0}")]
    Document(#[source] DocumentError),
    /// Reports a filesystem failure.
    #[error("filesystem operation failed")]
    Io(#[source] std::io::Error),
    /// Reports invalid or incomplete credentials.
    #[error("set both LXP_USERNAME and LXP_API_KEY, or select a local profile")]
    Credentials,
    /// Rejects invalid input paths.
    #[error("expected a regular PDF file or directory")]
    InvalidPath,
    /// Reports malformed invoice contents.
    #[error("invoice PDF data is missing or invalid")]
    InvalidInvoice,
    /// Reports output serialization failure.
    #[error("could not encode command output")]
    Json(#[source] serde_json::Error),
    /// Reports filesystem watch failure.
    #[error("filesystem watcher failed")]
    Watch(#[source] notify::Error),
    /// Avoids resubmitting a path after an earlier watcher attempt.
    #[error(
        "path has already been submitted in this watcher session; inspect jobs before restarting"
    )]
    RepeatedPath,
}

/// Runs a parsed API operation and prints structured results to stdout.
pub async fn run(api: &LxpApi, command: Command) -> Result<(), Error> {
    match command {
        Command::Balance => output(&api.balance().await.map_err(Error::Api)?),
        Command::Price { pages, print } => {
            let mut specification = Specification::from(&print);
            specification.pages = Some(pages.get());
            output(&api.price(specification).await.map_err(Error::Api)?)
        }
        Command::Jobs { filter, page } => {
            output(&api.jobs(filter, page.get()).await.map_err(Error::Api)?)
        }
        Command::Status { id } => output(&api.job(id).await.map_err(Error::Api)?),
        Command::Cancel { id } => {
            api.cancel(id).await.map_err(Error::Api)?;
            println!("Canceled job {id}");
            Ok(())
        }
        Command::Send(send) => send_path(api, &send).await,
        Command::WatchDir(send) => watch(api, &send).await,
        Command::Invoice { command } => match command {
            InvoiceCommand::List { page } => {
                output(&api.invoices(page.get()).await.map_err(Error::Api)?)
            }
            InvoiceCommand::Get { id, output } => {
                let invoice = api.invoice(id).await.map_err(Error::Api)?;
                let encoded = invoice.base64_data.ok_or(Error::InvalidInvoice)?;
                let pdf = STANDARD
                    .decode(encoded)
                    .map_err(|_| Error::InvalidInvoice)?;
                if !pdf.starts_with(b"%PDF-") {
                    return Err(Error::InvalidInvoice);
                }
                let mut options = tokio::fs::OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                options.mode(0o600);
                let mut file = options.open(output).await.map_err(Error::Io)?;
                file.write_all(&pdf).await.map_err(Error::Io)?;
                file.sync_all().await.map_err(Error::Io)
            }
        },
        Command::Profile { .. } => unreachable!("profiles are handled before API construction"),
    }
}

/// Prints a typed response without using diagnostics as the data channel.
fn output<T: Serialize>(value: &T) -> Result<(), Error> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(Error::Json)?
    );
    Ok(())
}

/// Submits directory entries sequentially and stops on the first unconfirmed job.
async fn send_path(api: &LxpApi, send: &Send) -> Result<(), Error> {
    let metadata = tokio::fs::symlink_metadata(&send.path)
        .await
        .map_err(Error::Io)?;
    if metadata.is_file() {
        return output(&send_one(api, send, &send.path).await?);
    }
    if !metadata.is_dir() {
        return Err(Error::InvalidPath);
    }
    let mut entries = tokio::fs::read_dir(&send.path).await.map_err(Error::Io)?;
    let mut paths = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(Error::Io)? {
        if entry.file_type().await.map_err(Error::Io)?.is_file() && is_pdf(&entry.path()) {
            paths.push(entry.path());
        }
    }
    paths.sort();
    if paths.is_empty() {
        return Err(Error::InvalidPath);
    }
    for path in paths {
        output(&send_one(api, send, &path).await?)?;
    }
    Ok(())
}

/// Reads a bounded PDF and makes exactly one application-level upload attempt.
async fn send_one(api: &LxpApi, send: &Send, path: &Path) -> Result<PrintJob, Error> {
    if !is_pdf(path)
        || !tokio::fs::symlink_metadata(path)
            .await
            .map_err(Error::Io)?
            .is_file()
    {
        return Err(Error::InvalidPath);
    }
    let file = tokio::fs::File::open(path).await.map_err(Error::Io)?;
    let mut pdf = Vec::new();
    file.take(MAX_PDF_BYTES as u64 + 1)
        .read_to_end(&mut pdf)
        .await
        .map_err(Error::Io)?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(Error::InvalidPath)?
        .to_owned();
    let letter = Letter::from_pdf(
        &pdf,
        filename,
        Specification::from(&send.print),
        send.notice.clone(),
    )
    .map_err(Error::Document)?;
    api.send(letter, send.yes).await.map_err(Error::Api)
}

/// Checks the filename extension without assuming UTF-8 paths.
fn is_pdf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
}

/// Watches completed writes and archives only confirmed submissions.
async fn watch(api: &LxpApi, send: &Send) -> Result<(), Error> {
    tokio::fs::create_dir_all(send.path.join("sent"))
        .await
        .map_err(Error::Io)?;
    let (tx, mut rx) = unbounded_channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = tx.send(event);
    })
    .map_err(Error::Watch)?;
    watcher
        .watch(&send.path, RecursiveMode::NonRecursive)
        .map_err(Error::Watch)?;
    let mut attempted = HashSet::new();
    while let Some(event) = rx.recv().await {
        let event = event.map_err(Error::Watch)?;
        if !matches!(
            event.kind,
            EventKind::Access(AccessKind::Close(AccessMode::Write))
                | EventKind::Modify(ModifyKind::Name(RenameMode::To))
        ) {
            continue;
        }
        for path in event.paths {
            if !is_pdf(&path) || !path.exists() {
                continue;
            }
            if !attempted.insert(path.clone()) {
                return Err(Error::RepeatedPath);
            }
            let job = send_one(api, send, &path).await?;
            output(&job)?;
            let filename = path.file_name().ok_or(Error::InvalidPath)?;
            let destination = send.path.join("sent").join(filename);
            tokio::fs::hard_link(&path, destination)
                .await
                .map_err(Error::Io)?;
            tokio::fs::remove_file(path).await.map_err(Error::Io)?;
        }
    }
    Ok(())
}
