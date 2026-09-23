//! Records submission intent before networking and retains ambiguous attempts.

use std::{
    fs,
    io::Write,
    num::NonZeroU64,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use hex_fmt::HexFmt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;

use crate::lxptypes::ApiMode;

/// Records whether the provider acknowledged an upload.
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum SubmissionState {
    /// Preserves an attempt that may or may not have reached the service.
    Pending,
    /// Identifies a positively acknowledged print job.
    Accepted {
        /// Identifies the provider's print job.
        job_id: NonZeroU64,
    },
}

/// Stores non-content metadata required to reconcile an upload.
#[derive(Debug, Deserialize, Serialize)]
struct Record {
    /// Separates shopping-cart and production submissions.
    mode: ApiMode,
    /// Tracks the acknowledgement state.
    #[serde(flatten)]
    state: SubmissionState,
}

/// Owns a durable attempt marker that is never deleted implicitly.
pub struct Reservation {
    /// Locates the receipt associated with this attempt.
    path: PathBuf,
    /// Identifies the processing mode of this attempt.
    mode: ApiMode,
    /// Correlates the attempt with a provider notice.
    reference: String,
}

/// Describes failures that must prevent an unrecorded submission.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Requires deliberate reconciliation before sending identical content again.
    #[error(
        "this PDF already has a submission receipt; inspect jobs before using --allow-duplicate"
    )]
    Duplicate,
    /// Reports receipt I/O failure.
    #[error("could not persist submission receipt")]
    Io(#[source] std::io::Error),
    /// Reports record encoding failure.
    #[error("could not encode submission receipt")]
    Json(#[source] serde_json::Error),
    /// Reports atomic receipt replacement failure.
    #[error("could not replace submission receipt")]
    Persist(#[source] tempfile::PersistError),
    /// Bounds repeated duplicate reservations.
    #[error("submission attempt counter exhausted")]
    Exhausted,
}

impl Reservation {
    /// Exclusively creates a durable pending marker before any HTTP upload.
    pub fn create(
        directory: &Path,
        account: &str,
        mode: ApiMode,
        pdf: &[u8],
        allow_duplicate: bool,
    ) -> Result<Self, Error> {
        fs::create_dir_all(directory).map_err(Error::Io)?;
        let mut digest = Sha256::new();
        digest.update(account.as_bytes());
        digest.update(b"\0");
        digest.update(match mode {
            ApiMode::Test => b"test",
            ApiMode::Live => b"live",
        });
        digest.update(b"\0");
        digest.update(pdf);
        let fingerprint = format!("{}", HexFmt(digest.finalize()));
        for attempt in 0..u32::MAX {
            let reference = format!("lxp-{fingerprint}-{attempt}");
            let path = directory.join(format!("{reference}.json"));
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            options.mode(0o600);
            let mut file = match options.open(&path) {
                Ok(file) => file,
                Err(error)
                    if error.kind() == std::io::ErrorKind::AlreadyExists && allow_duplicate =>
                {
                    continue;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    return Err(Error::Duplicate);
                }
                Err(error) => return Err(Error::Io(error)),
            };
            let record = Record {
                mode,
                state: SubmissionState::Pending,
            };
            serde_json::to_writer(&mut file, &record).map_err(Error::Json)?;
            file.write_all(b"\n").map_err(Error::Io)?;
            file.sync_all().map_err(Error::Io)?;
            sync_directory(directory)?;
            return Ok(Self {
                path,
                mode,
                reference,
            });
        }
        Err(Error::Exhausted)
    }

    /// Supplies the provider correlation reference without exposing document contents.
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// Atomically records acceptance, retaining the pending marker on failure.
    pub fn accept(self, job_id: NonZeroU64) -> Result<(), Error> {
        let directory = self
            .path
            .parent()
            .ok_or_else(|| Error::Io(std::io::Error::other("receipt has no parent")))?;
        let record = Record {
            mode: self.mode,
            state: SubmissionState::Accepted { job_id },
        };
        let mut file = NamedTempFile::new_in(directory).map_err(Error::Io)?;
        #[cfg(unix)]
        file.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(Error::Io)?;
        serde_json::to_writer(&mut file, &record).map_err(Error::Json)?;
        file.write_all(b"\n").map_err(Error::Io)?;
        file.as_file().sync_all().map_err(Error::Io)?;
        file.persist(&self.path).map_err(Error::Persist)?;
        sync_directory(directory)
    }
}

/// Persists directory entries where directory synchronization is supported.
fn sync_directory(directory: &Path) -> Result<(), Error> {
    #[cfg(unix)]
    fs::File::open(directory)
        .and_then(|file| file.sync_all())
        .map_err(Error::Io)?;
    #[cfg(not(unix))]
    let _ = directory;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, num::NonZeroU64};

    use super::{Error, Record, Reservation, SubmissionState};
    use crate::lxptypes::ApiMode;

    /// Retains pending attempts, blocks duplicates and records confirmed identifiers.
    #[test]
    fn durable_attempt_lifecycle() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let reserve = || {
            Reservation::create(
                directory.path(),
                "dummy",
                ApiMode::Test,
                b"%PDF-fixture",
                false,
            )
        };
        let first = reserve().expect("first reservation");
        let path = first.path.clone();
        assert!(matches!(reserve(), Err(Error::Duplicate)));
        drop(first);
        assert!(matches!(reserve(), Err(Error::Duplicate)));
        let pending: Record = serde_json::from_slice(&fs::read(&path).expect("read receipt"))
            .expect("decode receipt");
        assert!(matches!(pending.state, SubmissionState::Pending));
        let second = Reservation::create(
            directory.path(),
            "dummy",
            ApiMode::Test,
            b"%PDF-fixture",
            true,
        )
        .expect("explicit retry");
        let accepted_path = second.path.clone();
        second
            .accept(NonZeroU64::new(42).expect("positive id"))
            .expect("record acceptance");
        let accepted: Record =
            serde_json::from_slice(&fs::read(accepted_path).expect("read receipt"))
                .expect("decode receipt");
        assert!(
            matches!(accepted.state, SubmissionState::Accepted { job_id } if job_id.get() == 42)
        );
        assert!(
            Reservation::create(
                directory.path(),
                "dummy",
                ApiMode::Live,
                b"%PDF-fixture",
                false
            )
            .is_ok()
        );
        assert!(
            Reservation::create(
                directory.path(),
                "other-account",
                ApiMode::Test,
                b"%PDF-fixture",
                false
            )
            .is_ok()
        );
    }
}
