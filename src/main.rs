use clap;

use i3_ipc::{Connect, I3, I3Stream};

mod i3cache;
mod criteria;
mod search;
mod border;

use i3cache::I3Cache;

fn main() {
    let matches = clap::App::new(clap::crate_name!())
        .version(clap::crate_version!())
        .about(clap::crate_description!())
        .author(clap::crate_authors!())
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .arg(
            clap::Arg::with_name("criteria")
                .long("criteria")
                .short("c")
                .help("i3 command criteria for subsequent commands\n(terminate list with single ']' argument)")
                .takes_value(true)
                .default_value("[")
                .hide_default_value(true)
                .multiple(true)
                .value_terminator("]")
                .validator(criteria::validate_criteria),
        )
        .subcommand(
            clap::SubCommand::with_name("border")
                .about("Modify window border")
                .arg(
                    clap::Arg::with_name("toggle")
                        .long("toggle")
                        .short("t")
                        .help("Toggle between a list of border styles")
                        .takes_value(true)
                        .multiple(true)
                        .validator(border::validate_border),
                ),
        )
        .subcommand(clap::SubCommand::with_name("window").about("Find largest window"))
        .get_matches();

    let criteria: Vec<criteria::Match> = matches.values_of("criteria").map_or(vec![], |cr_args| {
        cr_args
            .filter_map(|cr| criteria::parse_criteria(cr).transpose())
            .collect::<Result<Vec<criteria::Match>, String>>()
            .unwrap() // already validated by clap
    });

    println!("Criteria: {:?}", criteria);

    let mut conn = I3::connect().unwrap();
    let data = I3Cache::new();

    match matches.subcommand() {
        ("border", Some(border_matches)) => border::border_subcmd(border_matches, &mut conn, &data),
        ("window", Some(window_matches)) => window_subcmd(window_matches, &mut conn, &data),
        _ => unreachable!(),
    }
}

fn window_subcmd(_matches: &clap::ArgMatches, conn: &mut I3Stream, data: &I3Cache) {
    //let tree = data.full_tree(conn).unwrap();
    //let workspaces = data.workspaces(conn).unwrap();
    let focused = data.focused_node(conn).unwrap();
    //let workspace = criteria::i3_find_focused_workspace(&workspaces, &tree).unwrap();
    let workspace = data.focused_workspace(conn).unwrap();
    let largest = search::i3_find_largest_tiled_window(&workspace).unwrap();

    println!("focused window: {:?}", focused.name);
    println!("focused workspace: {:?}", workspace.name);
    println!("largest window: {:?}", largest.name);
}
