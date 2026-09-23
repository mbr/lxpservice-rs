//! Runs explicit LetterXpress operations with nonzero failure exit codes.

#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

mod clidef;
mod logger;
mod lxpapi;
mod lxpcommands;
mod lxpconfig;
mod lxptypes;

use std::process::ExitCode;

use clap::Parser;
use sec::Secret;

use crate::{
    clidef::{Cli, Command, ProfileCommand},
    lxpapi::LxpApi,
    lxpcommands::Error,
    lxpconfig::LxpConfig,
};

/// Runs the selected command without printing credentials or server bodies on failure.
#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Err(error) = logger::init(cli.verbose) {
        eprintln!("could not initialize diagnostics: {error}");
        return ExitCode::FAILURE;
    }
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::warn!(error = %error, "operation failed");
            ExitCode::FAILURE
        }
    }
}

/// Resolves credentials and dispatches local or remote operations.
#[tracing::instrument(skip_all, level = "error")]
async fn run(cli: Cli) -> Result<(), Error> {
    let config_dir = dirs::config_dir().ok_or(Error::Credentials)?.join("lxp");
    if let Command::Profile { command } = cli.command {
        let mut config = LxpConfig::load(&config_dir).map_err(Error::Config)?;
        match command {
            ProfileCommand::List => config.list(),
            ProfileCommand::Save { name } => {
                let (username, apikey) = credentials(cli.username, cli.api_key, None)?;
                config
                    .save(
                        name,
                        lxpconfig::Profile {
                            user_name: username,
                            api_key: apikey,
                        },
                    )
                    .map_err(Error::Config)?;
            }
            ProfileCommand::Select { name } => config.select(name).map_err(Error::Config)?,
            ProfileCommand::Delete { name } => config.delete(&name).map_err(Error::Config)?,
        }
        return Ok(());
    }
    let profile = if cli.username.is_none() && cli.api_key.is_none() {
        LxpConfig::load(&config_dir)
            .map_err(Error::Config)?
            .active()
    } else {
        None
    };
    let (username, apikey) = credentials(cli.username, cli.api_key, profile)?;
    let api = LxpApi::new(username, apikey, cli.mode).map_err(Error::Api)?;
    lxpcommands::run(&api, cli.command).await
}

/// Resolves one complete credential source without mixing accounts.
fn credentials(
    username: Option<String>,
    apikey: Option<Secret<String>>,
    profile: Option<lxpconfig::Profile>,
) -> Result<(String, Secret<String>), Error> {
    let pair = match (username, apikey, profile) {
        (Some(username), Some(apikey), _) => (username, apikey),
        (None, None, Some(profile)) => (profile.user_name, profile.api_key),
        _ => return Err(Error::Credentials),
    };
    if pair.0.trim().is_empty() || pair.1.reveal().trim().is_empty() {
        return Err(Error::Credentials);
    }
    Ok(pair)
}
