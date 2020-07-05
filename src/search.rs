use i3_ipc::reply::{Node, NodeType, Workspaces};

#[derive(Eq, PartialEq, Clone, Copy)]
enum TreeIterPos {
    Parent,
    Node(usize),
    FloatingNode(usize),
    Done,
}

impl TreeIterPos {
    fn first(node: &Node) -> TreeIterPos {
        TreeIterPos::Parent.next(node)
    }

    fn next(&self, node: &Node) -> TreeIterPos {
        match self {
            TreeIterPos::Parent => {
                if node.nodes.len() > 0 {
                    TreeIterPos::Node(0)
                } else if node.floating_nodes.len() > 0 {
                    TreeIterPos::FloatingNode(0)
                } else {
                    TreeIterPos::Done
                }
            }
            TreeIterPos::Node(p) => {
                if node.nodes.len() > p + 1 {
                    TreeIterPos::Node(p + 1)
                } else if node.floating_nodes.len() > 0 {
                    TreeIterPos::FloatingNode(0)
                } else {
                    TreeIterPos::Done
                }
            }
            TreeIterPos::FloatingNode(p) => {
                if node.floating_nodes.len() > p + 1 {
                    TreeIterPos::FloatingNode(p + 1)
                } else {
                    TreeIterPos::Done
                }
            }
            TreeIterPos::Done => TreeIterPos::Done,
        }
    }
}

pub struct TreeIter<'a> {
    chain: Vec<&'a Node>,
    pos: Vec<TreeIterPos>,
}

impl<'a> From<&'a Node> for TreeIter<'a> {
    fn from(root: &'a Node) -> TreeIter<'a> {
        TreeIter {
            chain: vec![root],
            pos: vec![TreeIterPos::Parent],
        }
    }
}

impl<'a> Iterator for TreeIter<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        let tail = self.chain.last().unwrap();
        let tail_pos = *self.pos.last().unwrap();

        let next_node = match tail_pos {
            TreeIterPos::Parent => {
                *self.pos.last_mut().unwrap() = TreeIterPos::first(tail);
                return Some(tail);
            }
            TreeIterPos::Node(p) => &tail.nodes[p],
            TreeIterPos::FloatingNode(p) => &tail.floating_nodes[p],
            TreeIterPos::Done => {
                return None;
            }
        };

        let next_pos = TreeIterPos::first(next_node);
        self.chain.push(next_node);
        self.pos.push(next_pos);

        loop {
            let tail_pos = *self.pos.last().unwrap();

            if tail_pos == TreeIterPos::Done && self.chain.len() > 1 {
                self.chain.pop();
                self.pos.pop();

                let new_tail = self.chain.last().unwrap();
                let new_tail_pos = *self.pos.last().unwrap();

                *self.pos.last_mut().unwrap() = new_tail_pos.next(new_tail);

                continue;
            } else {
                break;
            }
        }

        Some(next_node)
    }
}

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
            let nn_size = nn.window_rect.width * nn.window_rect.height;
            let mm_size = mm.window_rect.width * mm.window_rect.height;
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
        .fold(None, |largest, node| match node.node_type {
            NodeType::Con => node.window.map_or_else(
                || i3_larger_node(largest, i3_find_largest_tiled_window(node)),
                |_w| i3_larger_node(largest, Some(node)),
            ),
            _ => i3_larger_node(largest, i3_find_largest_tiled_window(node)),
        })
}

pub fn i3_find_focused_workspace<'a>(workspaces: &Workspaces, tree: &'a Node) -> Option<&'a Node> {
    let workspace = workspaces
        .iter()
        .find(|w| w.focused == true)
        .unwrap()
        .name
        .as_str();
    i3_tree_find_first(tree, |n| {
        n.name.as_ref().map(|n| n.as_str()).unwrap_or("") == workspace
    })
}

pub fn i3_tree_find_first<P>(parent: &Node, mut predicate: P) -> Option<&Node>
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
