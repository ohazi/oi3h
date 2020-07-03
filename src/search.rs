use i3ipc::reply::{Node, NodeType, Workspaces};

pub fn i3_find_focused_node(parent: &Node) -> Option<&Node> {
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

pub fn i3_find_largest_tiled_window(parent: &Node) -> Option<&Node> {
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

pub fn i3_find_focused_workspace<'a>(workspaces: &Workspaces, tree: &'a Node) -> Option<&'a Node> {
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
