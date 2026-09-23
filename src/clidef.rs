//! Parses typed operations and explicit submission intent.

use std::{
    num::{NonZeroU32, NonZeroU64},
    path::PathBuf,
};

use clap::{Args, Parser, Subcommand};

use crate::lxptypes::{ApiMode, Color, JobFilter, Shipping, Sides, Specification};

/// Selects an authenticated client operation.
#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    /// Increases diagnostic verbosity.
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Identifies the LetterXpress account.
    #[arg(long, env = "LXP_USERNAME", global = true, hide_env_values = true)]
    pub username: Option<String>,
    /// Authenticates the account; prefer the environment over command-line arguments.
    #[arg(long, env = "LXP_API_KEY", global = true, hide_env_values = true)]
    pub api_key: Option<String>,
    /// Chooses test shopping-cart uploads or paid live processing.
    #[arg(
        long,
        env = "LXP_MODE",
        global = true,
        value_enum,
        default_value = "test"
    )]
    pub mode: ApiMode,
    /// Chooses the operation to perform.
    #[command(subcommand)]
    pub command: Command,
}

/// Defines client operations.
#[derive(Subcommand)]
pub enum Command {
    /// Shows available account credit.
    Balance,
    /// Estimates a price without uploading a document.
    Price {
        /// Gives the document page count.
        #[arg(long)]
        pages: NonZeroU32,
        /// Selects print and delivery options.
        #[command(flatten)]
        print: PrintOptions,
    },
    /// Lists one page of jobs; done means processed, not delivered.
    Jobs {
        /// Filters processing state.
        #[arg(long, value_enum)]
        filter: Option<JobFilter>,
        /// Selects the result page.
        #[arg(long, default_value = "1")]
        page: NonZeroU32,
    },
    /// Retrieves a job and its tracking information.
    Status {
        /// Identifies the print job.
        id: NonZeroU64,
    },
    /// Cancels a job within the provider's cancellation window.
    Cancel {
        /// Identifies the print job.
        id: NonZeroU64,
    },
    /// Submits a PDF or a directory of PDFs once, defaulting to test mode.
    #[command(alias = "set")]
    Send(Send),
    /// Watches completed PDF writes; stops on an unconfirmed submission.
    WatchDir(Send),
    /// Retrieves invoice metadata and PDFs.
    Invoice {
        /// Selects the invoice operation.
        #[command(subcommand)]
        command: InvoiceCommand,
    },
    /// Maintains legacy local profiles, separate from environment credentials.
    Profile {
        /// Selects profile maintenance.
        #[command(subcommand)]
        command: ProfileCommand,
    },
}

/// Selects invoice retrieval operations.
#[derive(Subcommand)]
pub enum InvoiceCommand {
    /// Lists a page of invoices.
    List {
        /// Selects the result page.
        #[arg(long, default_value = "1")]
        page: NonZeroU32,
    },
    /// Downloads a PDF without overwriting an existing file.
    Get {
        /// Identifies the invoice.
        id: NonZeroU64,
        /// Locates the new PDF file.
        #[arg(long)]
        output: PathBuf,
    },
}

/// Selects profile maintenance operations.
#[derive(Subcommand)]
pub enum ProfileCommand {
    /// Lists configured profiles without exposing keys.
    List,
    /// Saves environment credentials as a private local profile.
    Save {
        /// Names the profile.
        name: String,
    },
    /// Selects a stored profile.
    Select {
        /// Names the profile.
        name: String,
    },
    /// Deletes a stored profile.
    Delete {
        /// Names the profile.
        name: String,
    },
}

/// Selects printing independently of submission mode.
#[derive(Args)]
pub struct PrintOptions {
    /// Selects ink usage.
    #[arg(long, value_enum, default_value = "bw")]
    pub color: Color,
    /// Prints both sides of each sheet.
    #[arg(long)]
    pub duplex: bool,
    /// Selects the delivery region.
    #[arg(long, value_enum, default_value = "national")]
    pub shipping: Shipping,
}

impl From<&PrintOptions> for Specification {
    fn from(print: &PrintOptions) -> Self {
        Self {
            color: print.color,
            mode: if print.duplex {
                Sides::Duplex
            } else {
                Sides::Simplex
            },
            shipping: print.shipping,
            pages: None,
        }
    }
}

/// Selects documents and acknowledges paid submission when applicable.
#[derive(Args)]
pub struct Send {
    /// Locates the addressed PDF or directory.
    pub path: PathBuf,
    /// Selects print options.
    #[command(flatten)]
    pub print: PrintOptions,
    /// Confirms paid submission when the effective mode is live.
    #[arg(long)]
    pub yes: bool,
    /// Supplies a correlation reference, not an idempotency key.
    #[arg(long)]
    pub notice: Option<String>,
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::{Cli, Command};
    use crate::lxptypes::{ApiMode, Shipping, Sides, Specification};

    /// Validates mode precedence, positive identifiers and independent print flags.
    #[test]
    fn command_contract() {
        Cli::command().debug_assert();
        let cli = Cli::try_parse_from(["lxp", "--mode", "test", "send", "letter.pdf", "--duplex"])
            .expect("valid command");
        assert_eq!(cli.mode, ApiMode::Test);
        if let Command::Send(send) = cli.command {
            let specification = Specification::from(&send.print);
            assert!(matches!(specification.mode, Sides::Duplex));
            assert!(matches!(specification.shipping, Shipping::National));
            assert!(!send.yes);
        } else {
            panic!("expected send command");
        }
        assert!(Cli::try_parse_from(["lxp", "status", "0"]).is_err());
        assert!(Cli::try_parse_from(["lxp", "price", "--pages", "0"]).is_err());
        assert!(Cli::try_parse_from(["lxp", "--mode", "live", "send", "letter.pdf"]).is_ok());
    }
}
