use regex::Regex;

use i3_ipc::reply::{Node, NodeType, Output, Workspace};
use i3_ipc::I3Stream;

use crate::i3cache::I3Cache;
use crate::search;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowType {
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

// TODO: Use serde for this?
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Urgent {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConId {
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

#[derive(Debug, Clone)]
pub enum Match {
    Class(Regex),
    Instance(Regex),
    WindowRole(Regex),
    WindowType(WindowType),
    Id(u32),
    Title(Regex),
    Urgent(Urgent),
    Output(Regex),
    Workspace(Regex),
    ConMark(Regex),
    ConId(ConId),
    Floating,
    Tiling,
}

pub fn validate_criteria(criteria: String) -> Result<(), String> {
    parse_criteria(criteria.as_str())?;
    Ok(())
}

pub fn parse_criteria(input: &str) -> Result<Option<Match>, String> {
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
        "output" => token_split
            .get(1)
            .ok_or("output requires a parameter".to_string())
            .and_then(|param| {
                Regex::new(param)
                    .map(|r| Some(Match::Output(r)))
                    .map_err(|e| format!("output: {}", e))
            }),
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

/*
// TODO: ugh... probably easier to write the individual ones first
fn i3_criteria_search<'a>(
    conn: &mut I3Stream,
    data: &'a I3Cache,
    criteria: &[Match],
) -> Vec<&'a Node> {
    // Not sure how I want to implement this yet, but this needs to be a narrowing search, i.e.
    // first search is performed on the full tree, and subsequent searches are performed on this
    // list to remove non-matching nodes.
    let mut found = Vec::<&Node>::new();

    for c in criteria.iter() {
        match c {
            Match::Class(r) => {}
            Match::Instance(r) => {}
            Match::WindowRole(r) => {}
            Match::WindowType(wt) => {}
            Match::Id(id) => {
                let maybe_window_id =
                    search::i3_tree_find_first(data.full_tree(conn).unwrap(), |n| {
                        n.window == Some(*id)
                    });
                if let Some(id) = maybe_window_id {
                    found.push(id);
                }
            }
            Match::Title(r) => {}
            Match::Urgent(u) => {}
            Match::Output(p) => {}
            Match::Workspace(r) => {}
            Match::ConMark(Regex) => {}
            Match::ConId(ConId) => {}
            Match::Floating => {}
            Match::Tiling => {}
        }
    }

    found
}
*/

#[derive(Debug)]
pub struct OutputMatches<'a>(pub Vec<&'a Node>);

#[derive(Debug)]
pub struct WorkspaceMatches<'a>(pub Vec<&'a Node>);

#[derive(Debug)]
struct NodeMatches<'a>(Vec<&'a Node>);

pub fn all_outputs<'a>(conn: &mut I3Stream, data: &'a I3Cache) -> OutputMatches<'a> {
    let root = data.full_tree(conn).unwrap();

    let all_outputs = search::i3_tree_find_all(root, |n| n.node_type == NodeType::Output);
    OutputMatches(all_outputs)
}

pub fn match_output<'a>(
    conn: &mut I3Stream,
    data: &'a I3Cache,
    matches: OutputMatches<'a>,
    pattern: &Regex,
) -> OutputMatches<'a> {
    let outputs = data.outputs(conn).unwrap();

    // Some(Some(&Output)): A selected output that was found
    // Some(None):          A selected output that was not found
    // None:                A pattern
    let selection: Option<Option<&Output>> = match pattern.as_str() {
        "__focused__" | "__active__" => Some(outputs.iter().find(|o| o.active)),
        "__primary__" => Some(outputs.iter().find(|o| o.primary)),
        _ => None,
    };

    let new_matches: Vec<&Node> = match selection {
        Some(selection) => selection.map_or(vec![], |selected| {
            matches
                .0
                .iter()
                .filter_map(|node| {
                    if let Some(name) = &node.name {
                        if name.as_str() == selected.name.as_str() {
                            Some(*node)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect()
        }),
        None => matches
            .0
            .iter()
            .filter_map(|node| {
                if let Some(name) = &node.name {
                    if pattern.is_match(name.as_str()) {
                        Some(*node)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect(),
    };

    OutputMatches(new_matches)
}

pub fn all_workspaces<'a>(matches: OutputMatches<'a>) -> WorkspaceMatches<'a> {
    let all_workspaces: Vec<&Node> = matches
        .0
        .iter()
        .flat_map(|output| search::i3_tree_find_all(output, |n| n.node_type == NodeType::Workspace))
        .collect();

    WorkspaceMatches(all_workspaces)
}

pub fn match_workspace<'a>(
    conn: &mut I3Stream,
    data: &'a I3Cache,
    matches: WorkspaceMatches<'a>,
    pattern: &Regex,
) -> WorkspaceMatches<'a> {
    let workspaces = data.workspaces(conn).unwrap();

    // Some(Some(&Workspace)):  A selected workspace that was found
    // Some(None):              A selected workspace that was not found
    // None:                    A pattern
    let selection: Option<Option<&Workspace>> = match pattern.as_str() {
        "__focused__" => Some(workspaces.iter().find(|w| w.focused)),
        "__visible__" => Some(workspaces.iter().find(|w| w.visible)),
        "__urgent__" => Some(workspaces.iter().find(|w| w.urgent)),
        _ => None,
    };

    let new_matches: Vec<&Node> = match selection {
        Some(selection) => selection.map_or(vec![], |selected| {
            matches
                .0
                .iter()
                .filter_map(|node| {
                    if let Some(name) = &node.name {
                        if name.as_str() == selected.name.as_str() {
                            Some(*node)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect()
        }),
        None => matches
            .0
            .iter()
            .filter_map(|node| {
                if let Some(name) = &node.name {
                    if pattern.is_match(name.as_str()) {
                        Some(*node)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect(),
    };

    WorkspaceMatches(new_matches)
}
