//! Parses command-line operations.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Selects a client operation.
#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    /// Increases diagnostic verbosity.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Chooses the operation to perform.
    #[command(subcommand)]
    pub command: Command,
}

/// Defines supported client operations.
#[derive(Subcommand)]
pub enum Command {
    /// Creates and maintains profiles.
    Profile(Profile),
    /// Retrieves invoices.
    Invoice(Invoice),
    /// Inspects or deletes print jobs.
    Job(Job),
    /// Submits PDF files.
    Set(Send),
    /// Watches a directory for PDF files.
    WatchDir(Send),
}

/// Selects profile maintenance actions.
#[derive(Args)]
#[group(required = true, multiple = false, args = ["new", "delete", "delete_all", "switch", "overview"])]
pub struct Profile {
    /// Creates and selects a profile.
    #[arg(short, long, requires_all = ["profile", "user", "url", "api_key"])]
    pub new: bool,
    /// Deletes a profile.
    #[arg(short, long, requires = "profile")]
    pub delete: bool,
    /// Deletes every profile.
    #[arg(short = 'a', long = "delete_all")]
    pub delete_all: bool,
    /// Selects a profile.
    #[arg(short, long, requires = "profile")]
    pub switch: bool,
    /// Lists profiles.
    #[arg(short, long)]
    pub overview: bool,
    /// Supplies positional profile data.
    #[command(flatten)]
    pub data: ProfileData,
}

/// Supplies profile connection information.
#[derive(Args)]
#[group(skip)]
pub struct ProfileData {
    /// Names the profile.
    pub profile: Option<String>,
    /// Identifies the account.
    pub user: Option<String>,
    /// Sets the service URL.
    pub url: Option<String>,
    /// Authenticates the account.
    pub api_key: Option<String>,
}

/// Selects an invoice operation.
#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct Invoice {
    /// Retrieves an invoice by identifier.
    #[arg(short, long)]
    pub id: Option<i32>,
    /// Retrieves the latest invoice.
    #[arg(short, long)]
    pub current: bool,
    /// Lists invoices.
    #[arg(short, long)]
    pub list: bool,
}

/// Selects job operations.
#[derive(Args)]
pub struct Job {
    /// Enables cancellation.
    #[arg(short, long, requires = "selection", conflicts_with = "overview")]
    pub delete: bool,
    /// Cancels all pending jobs.
    #[arg(short, long, requires = "delete", group = "selection")]
    pub all: bool,
    /// Cancels a single job.
    #[arg(short, long, requires = "delete", group = "selection")]
    pub id: Option<i32>,
    /// Lists jobs.
    #[arg(short, long, required_unless_present = "delete")]
    pub overview: bool,
}

/// Selects documents and print options.
#[derive(Args)]
pub struct Send {
    /// Locates the PDF file or directory.
    pub path: PathBuf,
    /// Prints in black and white.
    #[arg(short, long = "black_and_white")]
    pub black_and_white: bool,
    /// Sends internationally.
    #[arg(short, long)]
    pub international: bool,
    /// Prints on both sides.
    #[arg(short, long)]
    pub duplex: bool,
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::Cli;

    /// Validates command constraints without running operations.
    #[test]
    fn command_constraints() {
        Cli::command().debug_assert();
        assert!(Cli::try_parse_from([
            "lxp",
            "profile",
            "--new",
            "home",
            "user",
            "https://example.com/",
            "key"
        ])
        .is_ok());
        assert!(Cli::try_parse_from(["lxp", "profile", "--new", "home"]).is_err());
        assert!(Cli::try_parse_from(["lxp", "job", "--delete"]).is_err());
        assert!(Cli::try_parse_from(["lxp", "job", "--delete", "--all", "--id", "1"]).is_err());
        assert!(Cli::try_parse_from(["lxp", "set", "letter.pdf", "--duplex"]).is_ok());
    }
}
