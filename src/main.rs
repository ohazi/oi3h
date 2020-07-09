use clap;

use i3_ipc::{Connect, I3Stream, I3};

mod border;
mod criteria;
mod i3cache;
mod search;

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
        .subcommand(clap::SubCommand::with_name("tree").about("Test"))
        .subcommand(clap::SubCommand::with_name("match").about("Test"))
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
        ("tree", Some(tree_matches)) => tree_subcmd(tree_matches, &mut conn, &data),
        ("match", Some(match_matches)) => match_subcmd(match_matches, &criteria, &mut conn, &data),
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

fn tree_subcmd(_matches: &clap::ArgMatches, conn: &mut I3Stream, data: &I3Cache) {
    let tree = data.full_tree(conn).unwrap();

    use search::TreeIter;

    for elem in TreeIter::from(tree) {
        println!("id: {}", elem.id);
    }
}

fn match_subcmd(
    _matches: &clap::ArgMatches,
    criteria: &[criteria::Match],
    conn: &mut I3Stream,
    data: &I3Cache,
) {
    let all_outputs = criteria::all_outputs(conn, data);
    println!(
        "all outputs: {:?}",
        all_outputs
            .0
            .iter()
            .map(|o| o.name.as_ref())
            .collect::<Vec<_>>()
    );

    let mut filtered_outputs = all_outputs;
    for oc in criteria.iter() {
        match oc {
            criteria::Match::Output(p) => {
                filtered_outputs = criteria::match_output(conn, data, filtered_outputs, p);
                println!("pattern: {}", p);
                //println!("filtered outputs: {:?}", filtered_outputs);
                println!(
                    "filtered outputs: {:?}",
                    filtered_outputs
                        .0
                        .iter()
                        .map(|o| o.name.as_ref())
                        .collect::<Vec<_>>()
                );
            }
            _ => {}
        }
    }

    let all_workspaces = criteria::all_workspaces(filtered_outputs);
    println!(
        "all workspaces on selected output(s): {:?}",
        all_workspaces
            .0
            .iter()
            .map(|w| w.name.as_ref())
            .collect::<Vec<_>>()
    );

    let mut filtered_workspaces = all_workspaces;
    for oc in criteria.iter() {
        match oc {
            criteria::Match::Workspace(p) => {
                filtered_workspaces = criteria::match_workspace(conn, data, filtered_workspaces, p);
                println!("pattern: {}", p);
                println!(
                    "filtered workspaces: {:?}",
                    filtered_workspaces
                        .0
                        .iter()
                        .map(|o| o.name.as_ref())
                        .collect::<Vec<_>>()
                );
            }
            _ => {}
        }
    }
}
