use clap;

use i3ipc::reply::{Node, NodeBorder};
use i3ipc::I3Connection;

use std::collections::HashSet;

fn main() {
    let matches = clap::App::new(clap::crate_name!())
        .version(clap::crate_version!())
        .about(clap::crate_description!())
        .author(clap::crate_authors!())
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            clap::SubCommand::with_name("border")
                .about("Modify window border")
                .arg(
                    clap::Arg::with_name("toggle")
                        .long("toggle")
                        .short("t")
                        .help("Toggle between specified list of border styles")
                        .takes_value(true)
                        .multiple(true)
                        .validator(border_validator),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("border", Some(toggle_matches)) => border_subcmd(toggle_matches),
        ("", None) => unreachable!(),
        _ => unreachable!(),
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum Border {
    None,
    Normal(Option<i32>),
    Pixel(Option<i32>),
}

fn border_parser(input: &str) -> Result<Border, String> {
    let mut tokens = input.split_whitespace();

    let first = tokens
        .next()
        .ok_or("Expected at least one token")?
        .to_lowercase();
    let second = tokens.next();
    let second: Option<i32> = match second.map(|s| s.parse::<i32>()) {
        Some(r) => Some(r.map_err(|_e| format!("'{}': {}", second.unwrap(), _e))?),
        None => None,
    };

    match first.as_str() {
        "none" => Ok(Border::None),
        "normal" => Ok(Border::Normal(second)),
        "pixel" => Ok(Border::Pixel(second)),
        s => Err(format!(
            "'{}': Expected one of: 'none', 'normal', 'pixel'",
            s
        )),
    }
}

fn border_validator(input: String) -> Result<(), String> {
    border_parser(input.as_str())?;
    Ok(())
}

fn border_subcmd(matches: &clap::ArgMatches) {
    if matches.is_present("toggle") {
        let toggle_states: Vec<Border> = matches
            .values_of("toggle")
            .unwrap()
            .map(|bs| border_parser(bs).unwrap()) // already validated by clap
            .collect();

        for border in toggle_states.iter() {
            println!("border: {:?}", border);
        }

        // toggle states should be unique
        // TODO: 'none' and 'pixel 0' might cause problems
        let toggle_states_set: HashSet<Border> = toggle_states.iter().cloned().collect();
        if toggle_states_set.len() != toggle_states.len() {
            eprintln!("Set of border states to toggle should be unique");
            std::process::exit(1);
        }

        let mut connection = I3Connection::connect().unwrap();

        let tree = connection.get_tree().unwrap();
        let focused = i3_find_focused_node(&tree).unwrap();

        // TODO: border_width seems to be in units of scaled pixels
        let border_width = focused.current_border_width;
        let current_border = match &focused.border {
            NodeBorder::None => Border::None,
            NodeBorder::Normal => Border::Normal(Some(border_width)),
            NodeBorder::Pixel => Border::Pixel(Some(border_width)),
            _ => Border::None,
        };

        println!("current border: {:?}", current_border);
    }
}

fn i3_find_focused_node(parent: &Node) -> Option<&Node> {
    if parent.focused {
        Some(parent)
    } else {
        if let Some(&focus) = parent.focus.get(0) {
            let child = parent.nodes.iter().find(|n| n.id == focus);
            child.map_or_else(
                || {
                    let floating_child = parent.floating_nodes.iter().find(|n| n.id == focus);
                    floating_child.map_or(None, |fc| i3_find_focused_node(fc))
                },
                |c| i3_find_focused_node(c),
            )
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_border_parser() {
        assert_eq!(border_parser("none"), Ok(Border::None));
        assert_eq!(border_parser("normal"), Ok(Border::Normal(None)));
        assert_eq!(border_parser("pixel"), Ok(Border::Pixel(None)));
        assert_eq!(border_parser("normal 2"), Ok(Border::Normal(Some(2))));
        assert_eq!(border_parser("pixel 2"), Ok(Border::Pixel(Some(2))));
    }
}
