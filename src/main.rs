//! Runs explicit LetterXpress operations with nonzero failure exit codes.

mod clidef;
mod logger;
mod lxpapi;
mod lxpcommands;
mod lxpconfig;
mod lxptypes;

use std::process::ExitCode;

use clap::Parser;

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
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Resolves credentials and dispatches local or remote operations.
async fn run(cli: Cli) -> Result<(), Error> {
    let config_dir = dirs::config_dir().ok_or(Error::Credentials)?.join("lxp");
    let log_dir = std::env::current_dir().map_err(Error::Io)?;
    logger::init("lxp", &log_dir, u64::from(cli.verbose));
    if let Command::Profile { command } = cli.command {
        let mut config = LxpConfig::new(&config_dir);
        match command {
            ProfileCommand::List => config.show_profiles(),
            ProfileCommand::Save { name } => {
                let (username, apikey) = credentials(cli.username, cli.api_key, None)?;
                config.new_profile(
                    &name,
                    lxpconfig::Profile {
                        user_name: username,
                        api_key: apikey,
                        url: "https://api.letterxpress.de/v3/".into(),
                    },
                );
            }
            ProfileCommand::Select { name } => config.switch_profile(&name),
            ProfileCommand::Delete { name } => config.delete_profile(&name),
        }
        return Ok(());
    }
    let profile = if cli.username.is_none() && cli.api_key.is_none() {
        LxpConfig::new(&config_dir).get_active_profile()
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
    apikey: Option<String>,
    profile: Option<lxpconfig::Profile>,
) -> Result<(String, String), Error> {
    let pair = match (username, apikey, profile) {
        (Some(username), Some(apikey), _) => (username, apikey),
        (None, None, Some(profile)) => (profile.user_name, profile.api_key),
        _ => return Err(Error::Credentials),
    };
    if pair.0.trim().is_empty() || pair.1.trim().is_empty() {
        return Err(Error::Credentials);
    }
    Ok(pair)
}
