//! Stores optional local profiles atomically without exposing credentials.

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use sec::Secret;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

/// Holds one complete account identity.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Profile {
    /// Identifies the account.
    pub user_name: String,
    /// Authenticates the account with redacted diagnostics.
    pub api_key: Secret<String>,
}

/// Stores named accounts and the selected identity.
#[derive(Debug, Default, Deserialize, Serialize)]
struct Profiles {
    /// Selects the account used when environment credentials are absent.
    profile_active: Option<String>,
    /// Maps local names to complete account identities.
    profiles: BTreeMap<String, Profile>,
}

/// Owns an explicitly loaded profile store.
pub struct LxpConfig {
    /// Locates the persistent configuration.
    path: PathBuf,
    /// Holds validated account selection and profile data.
    profiles: Profiles,
}

/// Describes configuration failures without logging file contents.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reports configuration I/O failure.
    #[error("could not access profile configuration")]
    Io(#[source] std::io::Error),
    /// Refuses malformed configuration instead of replacing it.
    #[error("profile configuration is malformed; refusing to overwrite it")]
    Decode {
        /// Identifies the malformed byte range without retaining credential text.
        span: Option<std::ops::Range<usize>>,
    },
    /// Reports serialization failure.
    #[error("could not encode profile configuration")]
    Encode(#[source] toml::ser::Error),
    /// Reports atomic replacement failure.
    #[error("could not atomically store profile configuration")]
    Persist(#[source] tempfile::PersistError),
    /// Rejects unknown or invalid profile selections.
    #[error("selected profile does not exist")]
    MissingProfile,
    /// Rejects empty local profile names.
    #[error("profile name must not be empty")]
    EmptyName,
}

impl LxpConfig {
    /// Loads a store without creating directories or silently resetting malformed data.
    pub fn load(directory: &Path) -> Result<Self, Error> {
        let path = directory.join("lxp.toml");
        let profiles: Profiles = match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text)
                .map_err(|error: toml::de::Error| Error::Decode { span: error.span() })?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Profiles::default(),
            Err(error) => return Err(Error::Io(error)),
        };
        if profiles
            .profile_active
            .as_ref()
            .is_some_and(|name| !profiles.profiles.contains_key(name))
        {
            return Err(Error::MissingProfile);
        }
        Ok(Self { path, profiles })
    }

    /// Returns the selected account without mixing credential sources.
    pub fn active(&self) -> Option<Profile> {
        self.profiles
            .profile_active
            .as_ref()
            .and_then(|name| self.profiles.profiles.get(name))
            .cloned()
    }

    /// Saves and selects a named account.
    pub fn save(&mut self, name: String, profile: Profile) -> Result<(), Error> {
        if name.trim().is_empty() {
            return Err(Error::EmptyName);
        }
        self.profiles.profiles.insert(name.clone(), profile);
        self.profiles.profile_active = Some(name);
        self.store()
    }

    /// Selects an existing profile.
    pub fn select(&mut self, name: String) -> Result<(), Error> {
        if !self.profiles.profiles.contains_key(&name) {
            return Err(Error::MissingProfile);
        }
        self.profiles.profile_active = Some(name);
        self.store()
    }

    /// Deletes a profile without unexpectedly switching an unrelated active account.
    pub fn delete(&mut self, name: &str) -> Result<(), Error> {
        if self.profiles.profiles.remove(name).is_none() {
            return Err(Error::MissingProfile);
        }
        if self.profiles.profile_active.as_deref() == Some(name) {
            self.profiles.profile_active = None;
        }
        self.store()
    }

    /// Lists names and selection state without exposing authentication material.
    pub fn list(&self) {
        for name in self.profiles.profiles.keys() {
            let marker = if self.profiles.profile_active.as_ref() == Some(name) {
                "*"
            } else {
                " "
            };
            println!("{marker} {name}");
        }
    }

    /// Atomically replaces the store with owner-only permissions on Unix.
    fn store(&self) -> Result<(), Error> {
        let directory = self.path.parent().ok_or(Error::MissingProfile)?;
        fs::create_dir_all(directory).map_err(Error::Io)?;
        let encoded = toml::to_string_pretty(&self.profiles).map_err(Error::Encode)?;
        let mut file = NamedTempFile::new_in(directory).map_err(Error::Io)?;
        #[cfg(unix)]
        file.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(Error::Io)?;
        file.write_all(encoded.as_bytes()).map_err(Error::Io)?;
        file.as_file().sync_all().map_err(Error::Io)?;
        file.persist(&self.path).map_err(Error::Persist)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use super::{Error, LxpConfig, Profile};

    /// Preserves selection, redacts secrets and stores profiles privately.
    #[test]
    fn profile_lifecycle() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut config = LxpConfig::load(directory.path()).expect("empty store");
        let profile = Profile {
            user_name: "dummy".into(),
            api_key: "test-secret".to_string().into(),
        };
        assert!(!format!("{profile:?}").contains("test-secret"));
        config
            .save("first".into(), profile.clone())
            .expect("save first");
        config.save("second".into(), profile).expect("save second");
        config.delete("first").expect("delete inactive");
        let reloaded = LxpConfig::load(directory.path()).expect("reload store");
        assert!(reloaded.active().is_some());
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(directory.path().join("lxp.toml"))
                .expect("profile metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        config.delete("second").expect("delete active");
        assert!(config.active().is_none());
    }

    /// Rejects malformed stores and dangling selections without modifying files.
    #[test]
    fn rejects_corrupt_configuration() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("lxp.toml");
        fs::write(&path, "not = [valid").expect("write fixture");
        assert!(matches!(
            LxpConfig::load(directory.path()),
            Err(Error::Decode { .. })
        ));
        assert_eq!(
            fs::read_to_string(&path).expect("read fixture"),
            "not = [valid"
        );
        fs::write(&path, "profile_active = 'missing'\n[profiles]\n").expect("write fixture");
        assert!(matches!(
            LxpConfig::load(directory.path()),
            Err(Error::MissingProfile)
        ));
    }
}
