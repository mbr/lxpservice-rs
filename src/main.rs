mod clidef;
mod logger;
mod lxpapi;
mod lxpcommands;
mod lxpconfig;
mod lxptypes;

use clap::{crate_name, crate_version};
use log::{debug, info};

#[tokio::main]
async fn main() {
    // Defenition of the command line interface
    let matches = clidef::cli_definition(crate_name!(), crate_version!());

    let verbose_level = matches.occurrences_of("verbose");

    let (log_dir, config_dir) = match matches.subcommand_matches("watch-dir") {
        Some(matches) => {
            let log_dir = std::fs::canonicalize(matches.value_of("directory").unwrap()) // CLAP ensures that
                .expect("Couldn't determine log_dir");
            let config_dir = std::path::PathBuf::from("/etc").join(crate_name!());
            (log_dir, config_dir)
        }
        None => {
            let log_dir = std::env::current_dir().expect("Couldn't determine log_dir");
            let config_dir = dirs::config_dir()
                .expect("Couldn't determine config_dir")
                .join(crate_name!());
            (log_dir, config_dir)
        }
    };

    logger::init(crate_name!(), &log_dir, verbose_level);
    info!("{} {}", crate_name!(), crate_version!());
    debug!("log_dir {:?}", log_dir);
    debug!("config_dir {:?}", config_dir);

    let mut lxp_cmds = lxpcommands::LxpCommands::new(&config_dir);

    // handle subcommand watch-dir
    if let Some(matches) = matches.subcommand_matches("watch-dir") {
        let color = match matches.is_present("black_and_white") {
            true => lxptypes::ColorPrint::BlackAndWhite,
            false => lxptypes::ColorPrint::Color,
        };
        let mode = match matches.is_present("international") {
            true => lxptypes::Mode::Duplex,
            false => lxptypes::Mode::Simplex,
        };
        let ship = match matches.is_present("duplex") {
            true => lxptypes::Ship::International,
            false => lxptypes::Ship::National,
        };
        //        let dir_name = &matches.value_of("directory").unwrap().to_string();
        lxp_cmds.watch_dir(&log_dir, color, mode, ship).await;
    }

    // handle subcommand profile
    if let Some(matches) = matches.subcommand_matches("profile") {
        if matches.is_present("new") {
            lxp_cmds.profile_new(
                matches.value_of("profile").unwrap(), // unwrap is ok, arg reqired
                matches.value_of("user").unwrap(),    // ...
                matches.value_of("url").unwrap(),
                matches.value_of("api_key").unwrap(),
            );
        }
        if matches.is_present("delete") {
            // unwrap is ok, arg reqired
            lxp_cmds.profile_delete(matches.value_of("profile").unwrap());
        }
        if matches.is_present("delete_all") {
            lxp_cmds.profile_delete_all();
        }
        if matches.is_present("switch") {
            // unwrap is ok, arg reqired
            lxp_cmds.profile_switch(matches.value_of("profile").unwrap());
        }
        if matches.is_present("overview") {
            lxp_cmds.profile_show();
        }
    }

    // handle subcommand invoice
    if let Some(matches) = matches.subcommand_matches("invoice") {
        if matches.is_present("list") {
            lxp_cmds.invoice_list().await;
        }
        if matches.is_present("current") {
            lxp_cmds.invoice_get_last().await;
        }
        if matches.is_present("id") {
            lxp_cmds
                .invoice_get_by_id(matches.value_of("id").unwrap())
                .await;
        };
    }

    // handle subcommand job
    if let Some(matches) = matches.subcommand_matches("job") {
        // show overview
        if matches.is_present("overview") {
            lxp_cmds.job_overview().await;
        }

        // delete job(s)
        if matches.is_present("delete") {
            if matches.is_present("all") {
                lxp_cmds.job_delete_all().await;
            } else {
                lxp_cmds
                    .job_delete_by_id(matches.value_of("id").unwrap())
                    .await;
            }
        }
    }

    // handle subcommand set
    if let Some(matches) = matches.subcommand_matches("set") {
        let color = match matches.is_present("black_and_white") {
            true => lxptypes::ColorPrint::BlackAndWhite,
            false => lxptypes::ColorPrint::Color,
        };
        let mode = match matches.is_present("international") {
            true => lxptypes::Mode::Duplex,
            false => lxptypes::Mode::Simplex,
        };
        let ship = match matches.is_present("duplex") {
            true => lxptypes::Ship::International,
            false => lxptypes::Ship::National,
        };
        let file_or_dir_name = matches.value_of("file_or_dir").unwrap().to_string();
        lxp_cmds
            .job_set_file_or_dir(&file_or_dir_name, color, mode, ship)
            .await;
    }
}
