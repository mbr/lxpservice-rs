// Some notes on error handling in logger.rs Since this library is only used in the context
// of the app lxp, errors are not returned but are handled directly in the sense of the app.
// This simplifies the interface design to the library.

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{collections::HashMap, fs, io::Write, path::PathBuf};

use clap::crate_name;
use log::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Profile {
    pub user_name: String,
    pub url: String,
    pub api_key: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Profiles {
    profile_active: Option<String>,
    profiles: HashMap<String, Profile>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct LxpConfig {
    config_path: PathBuf,
    profiles: Profiles,
}

impl LxpConfig {
    pub fn new(config_dir: &PathBuf) -> LxpConfig {
        if fs::create_dir_all(config_dir).is_err() {
            error!("Could not create config directory");
        }

        let mut config_path: PathBuf = config_dir.clone().join(crate_name!());
        config_path.set_extension("toml");

        let mut lxp_config = LxpConfig::default();

        let profiles = match fs::read_to_string(&config_path) {
            Ok(s) => match toml::from_str::<Profiles>(&s) {
                Ok(profiles) => profiles,
                Err(_) => {
                    error!("Invalid profile configuration; refusing to overwrite it");
                    return lxp_config;
                }
            },
            Err(_) => Profiles::default(),
        };

        lxp_config.config_path = config_path;
        lxp_config.profiles = profiles;
        lxp_config
    }

    fn store(&self) {
        match toml::to_string_pretty(&self.profiles) {
            Ok(toml_str) => {
                let mut options = fs::OpenOptions::new();
                options.write(true).create(true).truncate(true);
                #[cfg(unix)]
                options.mode(0o600);
                if options
                    .open(&self.config_path)
                    .and_then(|mut file| file.write_all(toml_str.as_bytes()))
                    .is_err()
                {
                    error!(
                        "LxpConfig: Can't write config to file, path {:#?}",
                        self.config_path
                    );
                }
            }
            Err(_) => error!("LxpConfig: Can't serialize config"),
        }
    }

    pub fn get_active_profile(&self) -> Option<Profile> {
        match &self.profiles.profile_active {
            Some(pa) => self.profiles.profiles.get(pa).cloned(),
            None => {
                error!("LxpConfig: no active profile found");
                None
            } // exits app
        }
    }

    pub fn get_active_profile_name(&self) -> Option<String> {
        self.profiles.profile_active.clone()
    }

    pub fn new_profile(&mut self, profile_name: &str, profile: Profile) {
        self.profiles.profiles.insert(profile_name.into(), profile);
        self.profiles.profile_active = Some(profile_name.into());
        self.store()
    }

    pub fn delete_all_profiles(&mut self) {
        self.profiles.profiles = HashMap::new();
        self.profiles.profile_active = None;
        self.store()
    }

    pub fn delete_profile(&mut self, profile_name: &str) {
        match self.profiles.profiles.remove(profile_name) {
            Some(_p) => {
                match self.profiles.profiles.keys().cloned().next() {
                    Some(pnew) => {
                        info!(
                            "Profile {} deleted, profile {} activated",
                            profile_name, pnew
                        );
                        self.profiles.profile_active = pnew.into();
                    }
                    None => {
                        info!(
                            "Profile {} deleted. No profile activated, because none is available",
                            profile_name
                        );
                        self.profiles.profile_active = None;
                    }
                };
                self.store()
            }
            None => error!("Could not delete profile {}: not found", profile_name), // exits app
        }
    }

    pub fn switch_profile(&mut self, profile_name: &str) {
        match self.profiles.profiles.get(profile_name) {
            Some(_v) => {
                info!("Active profile switched to '{}'", profile_name);
                self.profiles.profile_active = Some(profile_name.into())
            }
            None => error!("Could not switch to profile '{}': not found", profile_name), // exits app
        }
        self.store()
    }

    pub fn show_profiles(&self) {
        match &self.profiles.profile_active {
            Some(pn) => info!("Active profile '{}'", pn),
            None => info!("<No profile active>"),
        }
        info!("\n{:<15} {:<30} {}", "<profile>", "<user>", "<url>");
        for (profile_name, profile) in &self.profiles.profiles {
            info!(
                "{:<15} {:<30} {}",
                profile_name, profile.user_name, profile.url
            );
        }
    }
}
