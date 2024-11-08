use clap::{App, Arg, SubCommand};

pub fn subcommand() -> App<'static, 'static> {
    SubCommand::with_name("logwork")
        .about("Log work on a JIRA ticket")
        .arg(Arg::with_name("interactive")
            .short("i")
            .long("interactive")
            .help("Interactively log work on a ticket")
            .takes_value(false))       
        .arg(Arg::with_name("TICKET")
             .help("The ticket key or ID to log work against")
             .required(false)
             .index(1))
        .arg(Arg::with_name("TIME")
             .help("Time spent (e.g., 3h for 3 hours)")
             .required(false)
             .index(2))
        .arg(Arg::with_name("COMMENT")
             .help("Comment about the work log")
             .long("comment")
             .takes_value(true))
        .arg(Arg::with_name("START_TIME")
             .help("The start time when work was started (ISO 8601 format)")
             .long("start-time")
             .takes_value(true))
}
