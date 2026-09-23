use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

/// Define the command line interface
pub fn cli_definition(app_name: &str, version: &str) -> ArgMatches<'static> {
    App::new(app_name)
        .version(version)
        .author("Winfried Simon <winfried.simon@gmail.com>")
        .about("Command line tool to manage LetterXpress print jobs")
        .setting(AppSettings::ArgRequiredElseHelp)
        // Define flag verbose
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("Be communicative"),
        )
        // Define subcommand profile
        .subcommand(
            SubCommand::with_name("profile")
                .about("Create and maintain profiles")
                .after_help(
                    "A profile has a name and contains all information for accessing the web
service. With the subcommand profile they can be displayed, created and
deleted. You can also switch between them. 
",
                )
                .arg(
                    Arg::with_name("new")
                        .short("n")
                        .long("new")
                        .requires_all(&["profile", "user", "api_key"])
                        .help("Create and select a new profile"),
                )
                .arg(
                    Arg::with_name("delete")
                        .short("d")
                        .long("delete")
                        .requires("profile")
                        .help("Delete a single profile"),
                )
                .arg(
                    Arg::with_name("delete_all")
                        .short("a")
                        .long("delete_all")
                        .help("Delete all profiles"),
                )
                .arg(
                    Arg::with_name("switch")
                        .short("s")
                        .long("switch")
                        .requires("profile")
                        .help("Switch to profile"),
                )
                .arg(
                    Arg::with_name("overview")
                        .short("o")
                        .long("overview")
                        .help("Show all profiles"),
                )
                .arg(Arg::with_name("profile").help("Name of user profile"))
                .arg(Arg::with_name("user").help("User name of print service"))
                .arg(Arg::with_name("url").help("Url to print service"))
                .arg(Arg::with_name("api_key").help("Api key of print service")),
        )
        // Define subcommand invoice
        .subcommand(
            SubCommand::with_name("invoice")
                .about("Handle invoices")
                .after_help("List and get invoices.")
                .arg(
                    Arg::with_name("id")
                        .short("i")
                        .long("id")
                        .takes_value(true)
                        .help("Get invoice by id"),
                )
                .arg(
                    Arg::with_name("current")
                        .short("c")
                        .long("current")
                        .help("Get current (last) invoice"),
                )
                .arg(
                    Arg::with_name("list")
                        .short("l")
                        .long("list")
                        .help("Show list of available invoices"),
                ),
        )
        // Define subcommand job
        .subcommand(
            SubCommand::with_name("job")
                .about("Print job handling")
                .after_help("Show and delete print jobs.")
                .arg(
                    Arg::with_name("delete")
                        .short("d")
                        .long("delete")
                        .help("Delete print job on server"),
                )
                .arg(
                    Arg::with_name("all")
                        .short("a")
                        .long("all")
                        .requires("delete")
                        .help("Delete all print jobs on server"),
                )
                .arg(
                    Arg::with_name("id")
                        .short("i")
                        .long("id")
                        .takes_value(true)
                        .requires("delete")
                        .help("Delete print job by id"),
                )
                .arg(
                    Arg::with_name("overview")
                        .short("o")
                        .long("overview")
                        .help("Show informations about jobs on remote server"),
                ),
        )
        // Define subcommand set
        .subcommand(
            SubCommand::with_name("set")
                .about("Set print job(s) on server")
                .after_help("Set a single print job or many print jobs on server")
                .arg(
                    Arg::with_name("file_or_dir")
                        .required(true)
                        .help("PDF file or directory with PDF files"),
                )
                .arg(
                    Arg::with_name("black_and_white")
                        .short("b")
                        .long("black_and_white")
                        .help("Black and white print (default: color print)"),
                )
                .arg(
                    Arg::with_name("international")
                        .short("i")
                        .long("international")
                        .help("International destinations (default: national)"),
                )
                .arg(
                    Arg::with_name("duplex")
                        .short("d")
                        .long("duplex")
                        .help("Print on both sides (default: one side)"),
                ),
        )
        // Define subcommand set
        .subcommand(
            SubCommand::with_name("watch-dir")
                .about("Monitor a directory for letter orders")
                .after_help(
                    "PDF files saved in the monitored directory are then automatically uploaded as
a letter job. The PDF files are moved to the sent subdirectory
after the transfer. The parameters used to print and send the jobs are defined
in the call.

The profile definitions for access to the print service are expected under
/etc/lxp/lxp.toml. A log file is kept which is located in the monitored
directory.",
                )
                .arg(
                    Arg::with_name("directory")
                        .required(true)
                        .help("Supervised directory"),
                )
                .arg(
                    Arg::with_name("black_and_white")
                        .short("b")
                        .long("black_and_white")
                        .help("Black and white print (default: color print)"),
                )
                .arg(
                    Arg::with_name("international")
                        .short("i")
                        .long("international")
                        .help("International destinations (default: national)"),
                )
                .arg(
                    Arg::with_name("duplex")
                        .short("d")
                        .long("duplex")
                        .help("Print on both sides (default: one side)"),
                ),
        )
        .get_matches()
}
