use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/*
#[allow(dead_code)]
fn get_focused_con_id() -> Result<usize, String> {
    // TODO: We should probably only do this connection setup stuff once
    let mut connection = I3Connection::connect().map_err(|e| format!("{}", e))?;
    let tree = connection.get_tree().map_err(|e| format!("{}", e))?;
    let focused = i3_find_focused_node(&tree).ok_or("Unable to find focused node")?;

    // id is clearly a pointer, not sure why i3ipc thinks it's an i64...
    Ok(focused.id as usize)
}
*/

#[derive(Debug, Clone)]
pub enum Match {
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
