use clap;

use i3ipc::reply::{Node, NodeBorder, NodeType, Workspaces};
use i3ipc::I3Connection;

use std::collections::HashSet;
use std::hash::{Hash, Hasher};

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
                        .help("Toggle between a list of border styles")
                        .takes_value(true)
                        .multiple(true)
                        .validator(border_validator),
                ),
        )
        .subcommand(clap::SubCommand::with_name("window").about("Find largest window"))
        .get_matches();

    match matches.subcommand() {
        ("border", Some(border_matches)) => border_subcmd(border_matches),
        ("window", Some(window_matches)) => window_subcmd(window_matches),
        _ => unreachable!(),
    }
}

#[derive(Debug, Clone, Eq)]
struct Border {
    /// Only the `border` field will be considered when comparing `Border` values.
    /// The `width` field is not considered because users ask i3 to set the border
    /// width in pixels, but the i3 layout tree contains border width values in
    /// DPI-scaled pixels. Since there isn't an easy way to convert back, matching
    /// a requested state against the current state is difficult.
    border: NodeBorder,
    width: Option<i32>,
}

impl PartialEq for Border {
    fn eq(&self, other: &Self) -> bool {
        self.border == other.border
    }
}

impl Hash for Border {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.border.clone() as i32).hash(state);
    }
}

fn border_parser(input: &str) -> Result<Border, String> {
    let mut tokens = input.split_whitespace();
    let first = tokens
        .next()
        .ok_or("Expected at least one token")?
        .to_lowercase();
    let second = tokens.next();
    let second: Option<i32> = match second.map(|s| s.parse::<i32>()) {
        Some(r) => Some(r.map_err(|e| format!("'{}': {}", second.unwrap(), e))?),
        None => None,
    };

    match first.as_str() {
        "none" => Ok(Border {
            border: NodeBorder::None,
            width: None,
        }),
        "normal" => Ok(Border {
            border: NodeBorder::Normal,
            width: second,
        }),
        "pixel" => Ok(Border {
            border: NodeBorder::Pixel,
            width: second,
        }),
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
    let mut connection = I3Connection::connect().unwrap();
    let tree = connection.get_tree().unwrap();
    let focused = i3_find_focused_node(&tree).unwrap();

    // current_border_width seems to be in units of DPI-scaled pixels. There
    // doesn't appear to be an easy, robust way to convert back, so we'll only
    // match against the border type when cycling, and ignore the width. This
    // means that you won't be able to, e.g. toggle ["pixel 2" "pixel 5"
    // "pixel 10"], but you will be able to toggle ["none" "pixel 2" "normal 4"].
    let current_state = Border {
        border: focused.border.clone(),
        width: Some(focused.current_border_width),
    };

    if matches.is_present("toggle") {
        let toggle_states: Vec<Border> = matches
            .values_of("toggle")
            .unwrap()
            .map(|bs| border_parser(bs).unwrap()) // already validated by clap
            .collect();

        // toggle states should be unique
        // Note: i3 seems to differentiate between 'none' and 'pixel 0'
        // even though they are effectively identical.
        let toggle_states_set: HashSet<Border> = toggle_states.iter().cloned().collect();
        if toggle_states_set.len() != toggle_states.len() {
            eprintln!("Set of border states to toggle should be unique");
            std::process::exit(1);
        }

        // find index of current_state in toggle_states, otherwise use index 0
        let current_state_id: usize = toggle_states
            .iter()
            .enumerate()
            .find(|s| s.1 == &current_state)
            .map(|s| s.0)
            .unwrap_or(0);

        // pick the next state from toggle_states, wrapping around if necessary
        let next_state = toggle_states
            .iter()
            .cycle()
            .skip(current_state_id + 1)
            .next()
            .unwrap();

        let maybe_width: String = next_state
            .width
            .map(|w| w.to_string())
            .unwrap_or(String::from(""));

        match next_state.border {
            NodeBorder::None => {
                connection.run_command("border none").unwrap();
            }
            NodeBorder::Normal => {
                connection
                    .run_command(format!("border normal {}", maybe_width).as_str())
                    .unwrap();
            }
            NodeBorder::Pixel => {
                connection
                    .run_command(format!("border pixel {}", maybe_width).as_str())
                    .unwrap();
            }
            _ => {}
        }
    } else {
        println!("{:?}", current_state);
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

fn window_subcmd(_matches: &clap::ArgMatches) {
    let mut connection = I3Connection::connect().unwrap();
    let tree = connection.get_tree().unwrap();
    let workspaces = connection.get_workspaces().unwrap();
    let focused = i3_find_focused_node(&tree).unwrap();
    let workspace = i3_find_focused_workspace(&workspaces, &tree).unwrap();
    let largest = i3_find_largest_tiled_window(&workspace).unwrap();

    println!("focused window: {:?}", focused.name);
    println!("focused workspace: {:?}", workspace.name);
    println!("largest window: {:?}", largest.name);
}

fn i3_larger_node<'a>(n: Option<&'a Node>, m: Option<&'a Node>) -> Option<&'a Node> {
    m.map_or(n, |mm| {
        n.map_or(m, |nn| {
            let nn_size = nn.window_rect.2 * nn.window_rect.3;
            let mm_size = mm.window_rect.2 * mm.window_rect.3;
            if nn_size > mm_size {
                n
            } else {
                m
            }
        })
    })
}

fn i3_find_largest_tiled_window(parent: &Node) -> Option<&Node> {
    parent
        .nodes
        .iter()
        .fold(None, |largest, node| match node.nodetype {
            NodeType::Con => node.window.map_or_else(
                || i3_larger_node(largest, i3_find_largest_tiled_window(node)),
                |_w| i3_larger_node(largest, Some(node)),
            ),
            _ => i3_larger_node(largest, i3_find_largest_tiled_window(node)),
        })
}

fn i3_find_focused_workspace<'a>(workspaces: &Workspaces, tree: &'a Node) -> Option<&'a Node> {
    let workspace = workspaces.workspaces.iter().find(|w| w.focused == true).unwrap().name.as_str();
    i3_tree_find(tree, &|n: &&Node| {
        n.name.as_ref().map(|n| n.as_str()).unwrap_or("") == workspace
    })
}

fn i3_tree_find<'a, P>(parent: &'a Node, predicate: &P) -> Option<&'a Node> 
    where P: Fn(&&Node) -> bool,
{
    if predicate(&parent) {
        Some(parent)
    } else {
        for child in &parent.nodes {
            let res = i3_tree_find(child, predicate);
            if res.is_some() {
                return res;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_border_parser() {
        assert_eq!(
            border_parser("none"),
            Ok(Border {
                border: NodeBorder::None,
                width: None
            })
        );
        assert_eq!(
            border_parser("normal"),
            Ok(Border {
                border: NodeBorder::Normal,
                width: None
            })
        );
        assert_eq!(
            border_parser("pixel"),
            Ok(Border {
                border: NodeBorder::Pixel,
                width: None
            })
        );
        assert_eq!(
            border_parser("normal 2"),
            Ok(Border {
                border: NodeBorder::Normal,
                width: Some(2)
            })
        );
        assert_eq!(
            border_parser("pixel 2"),
            Ok(Border {
                border: NodeBorder::Pixel,
                width: Some(2)
            })
        );
    }
}
