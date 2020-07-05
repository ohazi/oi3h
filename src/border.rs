use i3_ipc::reply::NodeBorder;
use i3_ipc::I3Stream;

use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::i3cache::I3Cache;

#[derive(Debug, Clone, Eq)]
pub struct Border {
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

pub fn parse_border(input: &str) -> Result<Border, String> {
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

pub fn validate_border(border: String) -> Result<(), String> {
    parse_border(border.as_str())?;
    Ok(())
}

pub fn border_subcmd(matches: &clap::ArgMatches, conn: &mut I3Stream, data: &I3Cache) {
    //let criteria = matches.value_of("criteria").unwrap();
    let criteria = "";

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
    let focused = data.focused_node(conn).unwrap();
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
                conn.run_command(format!("[{}] border none", criteria).as_str())
                    .unwrap();
            }
            NodeBorder::Normal => {
                conn.run_command(format!("[{}] border normal {}", criteria, maybe_width).as_str())
                    .unwrap();
            }
            NodeBorder::Pixel => {
                conn.run_command(format!("[{}] border pixel {}", criteria, maybe_width).as_str())
                    .unwrap();
            }
        }
    } else {
        println!("{:?}", current_state);
    }
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
