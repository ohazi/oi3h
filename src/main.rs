use clap;

use i3ipc::reply::{Node, NodeBorder, NodeType, Workspaces};
use i3ipc::I3Connection;

use regex::Regex;

use std::collections::HashSet;
use std::hash::{Hash, Hasher};

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
                .validator(validate_criteria),
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
                        .validator(validate_border),
                ),
        )
        .subcommand(clap::SubCommand::with_name("window").about("Find largest window"))
        .get_matches();

    let criteria: Vec<Match> = matches.values_of("criteria").map_or(vec![], |cr_args| {
        cr_args
            .filter_map(|cr| parse_criteria(cr).transpose())
            .collect::<Result<Vec<Match>, String>>()
            .unwrap() // already validated by clap
    });

    println!("Criteria: {:?}", criteria);

    match matches.subcommand() {
        ("border", Some(border_matches)) => border_subcmd(border_matches),
        ("window", Some(window_matches)) => window_subcmd(window_matches),
        _ => unreachable!(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowType {
    Normal,
    Dialog,
    Utility,
    Toolbar,
    Splash,
    Menu,
    DropdownMenu,
    PopupMenu,
    Tooltip,
    Notification,
}

fn parse_window_type(input: &str) -> Result<WindowType, String> {
    match input.to_lowercase().as_str() {
        "normal" => Ok(WindowType::Normal),
        "dialog" => Ok(WindowType::Dialog),
        "utility" => Ok(WindowType::Utility),
        "toolbar" => Ok(WindowType::Toolbar),
        "splash" => Ok(WindowType::Splash),
        "menu" => Ok(WindowType::Menu),
        "dropdown_menu" => Ok(WindowType::DropdownMenu),
        "popup_menu" => Ok(WindowType::PopupMenu),
        "tooltip" => Ok(WindowType::Tooltip),
        "notification" => Ok(WindowType::Notification),
        s => Err(format!("Unknown window_type: '{}'", s)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Urgent {
    Latest,
    Oldest,
}

fn parse_urgent(input: &str) -> Result<Urgent, String> {
    match input.to_lowercase().as_str() {
        "latest" | "newest" | "recent" | "last" => Ok(Urgent::Latest),
        "oldest" | "first" => Ok(Urgent::Oldest),
        s => Err(format!("Unknown urgency: '{}'", s)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConId {
    Focused,
    Id(usize),
}

fn parse_con_id(input: &str) -> Result<ConId, String> {
    match input {
        "__focused__" => Ok(ConId::Focused),
        param => match param.starts_with("0x") {
            true => usize::from_str_radix(&param[2..], 16),
            false => param.parse(),
        }
        .map(|parsed| ConId::Id(parsed))
        .map_err(|e| format!("con_id: {}", e)),
    }
}

#[allow(dead_code)]
fn get_focused_con_id() -> Result<usize, String> {
    // TODO: We should probably only do this connection setup stuff once
    let mut connection = I3Connection::connect().map_err(|e| format!("{}", e))?;
    let tree = connection.get_tree().map_err(|e| format!("{}", e))?;
    let focused = i3_find_focused_node(&tree).ok_or("Unable to find focused node")?;

    // id is clearly a pointer, not sure why i3ipc thinks it's an i64...
    Ok(focused.id as usize)
}

#[derive(Debug, Clone)]
enum Match {
    Class(Regex),
    Instance(Regex),
    WindowRole(Regex),
    WindowType(WindowType),
    Id(u32),
    Title(Regex),
    Urgent(Urgent),
    Workspace(Regex),
    ConMark(Regex),
    ConId(ConId),
    Floating,
    Tiling,
}

fn validate_criteria(criteria: String) -> Result<(), String> {
    parse_criteria(criteria.as_str())?;
    Ok(())
}

fn parse_criteria(input: &str) -> Result<Option<Match>, String> {
    let mut token_split: Vec<&str> = input.splitn(2, '=').collect();
    if let Some(param) = token_split.get_mut(1) {
        // for compatibility with i3 criteria...
        if param.starts_with('"') && param.ends_with('"') && param.len() > 1 {
            *param = param.get(1..(param.len() - 1)).unwrap();
        }
    }
    let token_split = token_split;
    match token_split[0].to_lowercase().as_str() {
        "[" => Ok(None),
        "]" => Ok(None), // Shouldn't need this with clap terminator, but for completeness...
        "class" => token_split
            .get(1)
            .ok_or("class requires a parameter".to_string())
            .and_then(|param| {
                Regex::new(param)
                    .map(|r| Some(Match::Class(r)))
                    .map_err(|e| format!("class: {}", e))
            }),
        "instance" => token_split
            .get(1)
            .ok_or("instance requires a parameter".to_string())
            .and_then(|param| {
                Regex::new(param)
                    .map(|r| Some(Match::Instance(r)))
                    .map_err(|e| format!("instance: {}", e))
            }),
        "window_role" => token_split
            .get(1)
            .ok_or("window_role requires a parameter".to_string())
            .and_then(|param| {
                Regex::new(param)
                    .map(|r| Some(Match::WindowRole(r)))
                    .map_err(|e| format!("window_role: {}", e))
            }),
        "window_type" => token_split
            .get(1)
            .ok_or("window_type requires a parameter".to_string())
            .and_then(|param| parse_window_type(param).map(|wt| Some(Match::WindowType(wt)))),
        "id" => token_split
            .get(1)
            .ok_or("id requires a parameter".to_string())
            .and_then(|param| {
                match param.starts_with("0x") {
                    true => u32::from_str_radix(&param[2..], 16),
                    false => param.parse(),
                }
                .map(|parsed| Some(Match::Id(parsed)))
                .map_err(|e| format!("id: {}", e))
            }),
        "title" => token_split
            .get(1)
            .ok_or("title requires a parameter".to_string())
            .and_then(|param| {
                Regex::new(param)
                    .map(|r| Some(Match::Title(r)))
                    .map_err(|e| format!("title: {}", e))
            }),
        "urgent" => token_split
            .get(1)
            .ok_or("urgent requires a parameter".to_string())
            .and_then(|param| parse_urgent(param).map(|u| Some(Match::Urgent(u)))),
        "workspace" => token_split
            .get(1)
            .ok_or("workspace requires a parameter".to_string())
            .and_then(|param| {
                Regex::new(param)
                    .map(|r| Some(Match::Workspace(r)))
                    .map_err(|e| format!("workspace: {}", e))
            }),
        "con_mark" => token_split
            .get(1)
            .ok_or("con_mark requires a parameter".to_string())
            .and_then(|param| {
                Regex::new(param)
                    .map(|r| Some(Match::ConMark(r)))
                    .map_err(|e| format!("con_mark: {}", e))
            }),
        "con_id" => token_split
            .get(1)
            .ok_or("con_id requires a parameter".to_string())
            .and_then(|param| parse_con_id(param).map(|ci| Some(Match::ConId(ci)))),
        "floating" => Ok(Some(Match::Floating)),
        "tiling" => Ok(Some(Match::Tiling)),
        _ => Err(format!("Unknown criteria: '{}'", input)),
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

fn parse_border(input: &str) -> Result<Border, String> {
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

fn validate_border(border: String) -> Result<(), String> {
    parse_border(border.as_str())?;
    Ok(())
}

fn border_subcmd(matches: &clap::ArgMatches) {
    let mut connection = I3Connection::connect().unwrap();
    let tree = connection.get_tree().unwrap();
    let focused = i3_find_focused_node(&tree).unwrap();

    let criteria = matches.value_of("criteria").unwrap();

    // TODO: should current_state always come from focused window, or does it
    // ever make sense to use a different window based on the command criteria?
    // i3's border command performs the criteria match first, then performs the
    // toggle on each matching node individually. I don't think we can do that
    // easily if we want to reuse i3's criteria matching system.
    //
    // An example of when a user may want different behavior is if they want a
    // binding to toggle the borders on floating windows:
    // bindsym $mod+t exec --no-startup-id "oi3h -c floating border -t normal pixel"
    //
    // This wouldn't work unless a floating window happens to be focused when
    // the keybinding is used, which may be annoying. i3's border toggle
    // command would work correctly in this case.
    //
    // A robust solution would require re-implementing command criteria in
    // order to find the matching nodes here. It would then be possible to run
    // a specific command on each node based on its current state rather than
    // one command on all nodes based on the focused window's current state.

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
            .map(|bs| parse_border(bs).unwrap()) // already validated by clap
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
            .unwrap_or("".to_string());

        match next_state.border {
            NodeBorder::None => {
                connection
                    .run_command(format!("[{}] border none", criteria).as_str())
                    .unwrap();
            }
            NodeBorder::Normal => {
                connection
                    .run_command(format!("[{}] border normal {}", criteria, maybe_width).as_str())
                    .unwrap();
            }
            NodeBorder::Pixel => {
                connection
                    .run_command(format!("[{}] border pixel {}", criteria, maybe_width).as_str())
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
    let workspace = workspaces
        .workspaces
        .iter()
        .find(|w| w.focused == true)
        .unwrap()
        .name
        .as_str();
    i3_tree_find_first(tree, |n| {
        n.name.as_ref().map(|n| n.as_str()).unwrap_or("") == workspace
    })
}

fn i3_tree_find_first<P>(parent: &Node, mut predicate: P) -> Option<&Node>
where
    P: FnMut(&Node) -> bool,
{
    i3_tree_find_first_helper(parent, &mut predicate)
}

fn i3_tree_find_first_helper<'a, P>(parent: &'a Node, predicate: &mut P) -> Option<&'a Node>
where
    P: FnMut(&Node) -> bool,
{
    if predicate(parent) {
        Some(parent)
    } else {
        for child in parent.nodes.iter() {
            let res = i3_tree_find_first_helper(child, predicate);
            if res.is_some() {
                return res;
            }
        }
        for child in parent.floating_nodes.iter() {
            let res = i3_tree_find_first_helper(child, predicate);
            if res.is_some() {
                return res;
            }
        }
        None
    }
}

#[allow(dead_code)]
fn i3_tree_find_all<P>(parent: &Node, mut predicate: P) -> Vec<&Node>
where
    P: FnMut(&Node) -> bool,
{
    let res: Vec<&Node> = vec![];
    i3_tree_find_all_helper(parent, &mut predicate, res)
}

fn i3_tree_find_all_helper<'a, P>(
    parent: &'a Node,
    predicate: &mut P,
    mut res: Vec<&'a Node>,
) -> Vec<&'a Node>
where
    P: FnMut(&Node) -> bool,
{
    for child in parent.nodes.iter() {
        res = i3_tree_find_all_helper(child, predicate, res);
    }
    for child in parent.floating_nodes.iter() {
        res = i3_tree_find_all_helper(child, predicate, res);
    }
    if predicate(parent) {
        res.push(parent);
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_border() {
        assert_eq!(
            parse_border("none"),
            Ok(Border {
                border: NodeBorder::None,
                width: None
            })
        );
        assert_eq!(
            parse_border("normal"),
            Ok(Border {
                border: NodeBorder::Normal,
                width: None
            })
        );
        assert_eq!(
            parse_border("pixel"),
            Ok(Border {
                border: NodeBorder::Pixel,
                width: None
            })
        );
        assert_eq!(
            parse_border("normal 2"),
            Ok(Border {
                border: NodeBorder::Normal,
                width: Some(2)
            })
        );
        assert_eq!(
            parse_border("pixel 2"),
            Ok(Border {
                border: NodeBorder::Pixel,
                width: Some(2)
            })
        );
    }
}
