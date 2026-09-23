//! Runs the LetterXpress command-line client.

mod clidef;
mod logger;
mod lxpapi;
mod lxpcommands;
mod lxpconfig;
mod lxptypes;

use clap::Parser;

use crate::clidef::{Cli, Command};

/// Dispatches parsed operations.
#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let log_dir = std::env::current_dir().expect("current directory is available");
    let config_dir = if matches!(cli.command, Command::WatchDir(_)) {
        std::path::PathBuf::from("/etc/lxp")
    } else {
        dirs::config_dir()
            .expect("configuration directory is available")
            .join("lxp")
    };
    logger::init("lxp", &log_dir, u64::from(cli.verbose));
    let mut commands = lxpcommands::LxpCommands::new(&config_dir);
    match cli.command {
        Command::Profile(profile) => {
            let data = profile.data;
            if profile.new {
                commands.profile_new(
                    data.profile.as_deref().expect("clap requires profile"),
                    data.user.as_deref().expect("clap requires user"),
                    data.url.as_deref().expect("clap requires url"),
                    data.api_key.as_deref().expect("clap requires key"),
                );
            } else if profile.delete {
                commands.profile_delete(data.profile.as_deref().expect("clap requires profile"));
            } else if profile.delete_all {
                commands.profile_delete_all();
            } else if profile.switch {
                commands.profile_switch(data.profile.as_deref().expect("clap requires profile"));
            } else {
                commands.profile_show();
            }
        }
        Command::Invoice(invoice) => {
            if let Some(id) = invoice.id {
                commands.invoice_get_by_id(&id.to_string()).await;
            } else if invoice.current {
                commands.invoice_get_last().await;
            } else {
                commands.invoice_list().await;
            }
        }
        Command::Job(job) => {
            if let Some(id) = job.id {
                commands.job_delete_by_id(&id.to_string()).await;
            } else if job.all {
                commands.job_delete_all().await;
            } else {
                commands.job_overview().await;
            }
        }
        Command::Set(send) => {
            let (color, mode, ship) = print_options(&send);
            commands
                .job_set_file_or_dir(&send.path.to_string_lossy(), color, mode, ship)
                .await;
        }
        Command::WatchDir(send) => {
            let (color, mode, ship) = print_options(&send);
            commands.watch_dir(&send.path, color, mode, ship).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::print_options;
    use crate::{
        clidef::Send,
        lxptypes::{Mode, Ship},
    };

    /// Keeps duplex and international flags independent.
    #[test]
    fn independent_print_flags() {
        let mut send = Send {
            path: PathBuf::from("test.pdf"),
            black_and_white: true,
            international: false,
            duplex: true,
        };
        assert!(matches!(
            print_options(&send),
            (_, Mode::Duplex, Ship::National)
        ));
        send.duplex = false;
        send.international = true;
        assert!(matches!(
            print_options(&send),
            (_, Mode::Simplex, Ship::International)
        ));
    }
}

/// Converts print flags into API options.
fn print_options(send: &clidef::Send) -> (lxptypes::ColorPrint, lxptypes::Mode, lxptypes::Ship) {
    (
        if send.black_and_white {
            lxptypes::ColorPrint::BlackAndWhite
        } else {
            lxptypes::ColorPrint::Color
        },
        if send.duplex {
            lxptypes::Mode::Duplex
        } else {
            lxptypes::Mode::Simplex
        },
        if send.international {
            lxptypes::Ship::International
        } else {
            lxptypes::Ship::National
        },
    )
}
