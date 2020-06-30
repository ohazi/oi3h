use i3ipc::reply::{Node, Outputs, Workspaces};
use i3ipc::I3Connection;
use i3ipc::MessageError;

use std::marker::PhantomPinned;
use std::pin::Pin;

use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use crate::criteria;

struct I3Nodes {
    full_tree: Node,
    focused_node: *const Node,
    focused_workspace: *const Node,
    _pin: PhantomPinned,
}

pub struct I3Data {
    nodes: Cell<Option<Pin<Box<I3Nodes>>>>,
    workspaces: RefCell<Option<Rc<Workspaces>>>,
    outputs: RefCell<Option<Rc<Outputs>>>,
}

impl I3Data {
    pub fn new() -> I3Data {
        I3Data {
            nodes: Cell::new(None),
            workspaces: RefCell::new(None),
            outputs: RefCell::new(None),
        }
    }

    fn nodes(&self) -> Option<&I3Nodes> {
        let orpb_i3n: Option<&Pin<Box<I3Nodes>>> = unsafe { (*self.nodes.as_ptr()).as_ref() };
        let or_i3n: Option<&I3Nodes> = orpb_i3n.map(|i| i.as_ref().get_ref());
        or_i3n
    }

    fn nodes_mut(&self) -> Option<Pin<&mut I3Nodes>> {
        let borrow_mut: Option<&mut Pin<Box<I3Nodes>>> = unsafe { (*self.nodes.as_ptr()).as_mut() };
        let mut_ref: Option<Pin<&mut I3Nodes>> = borrow_mut.map(|i| Pin::as_mut(i));
        mut_ref
    }

    pub fn full_tree(&self, conn: &mut I3Connection) -> Result<&Node, MessageError> {
        if self.nodes().is_none() {
            self.nodes.set(Some(Box::pin(I3Nodes {
                full_tree: conn.get_tree()?,
                focused_node: std::ptr::null(),
                focused_workspace: std::ptr::null(),
                _pin: PhantomPinned,
            })));
        }
        Ok(&self.nodes().unwrap().full_tree)
    }

    pub fn focused_node(&self, conn: &mut I3Connection) -> Result<&Node, String> {
        self.full_tree(conn).map_err(|e| format!("{}", e))?;
        if self.nodes().unwrap().focused_node.is_null() {
            let nodes_mut = self.nodes_mut().unwrap();
            let full_tree = &nodes_mut.full_tree;
            unsafe {
                Pin::get_unchecked_mut(nodes_mut).focused_node =
                    criteria::i3_find_focused_node(full_tree)
                        .ok_or("Unable to find focused node")?;
            }
        }
        let focused_node: &Node = unsafe { self.nodes().unwrap().focused_node.as_ref().unwrap() };
        Ok(focused_node)
    }

    #[allow(dead_code)]
    pub fn focused_workspace(&self, conn: &mut I3Connection) -> Result<&Node, String> {
        let tree = self.full_tree(conn).map_err(|e| format!("{}", e))?;
        let workspaces = self.workspaces(conn).map_err(|e| format!("{}", e))?;
        if self.nodes().unwrap().focused_workspace.is_null() {
            let nodes_mut = self.nodes_mut().unwrap();
            unsafe {
                Pin::get_unchecked_mut(nodes_mut).focused_workspace =
                    criteria::i3_find_focused_workspace(&workspaces, tree)
                        .ok_or("Unable to find focused workspace")?;
            }
        }
        let focused_workspace: &Node =
            unsafe { self.nodes().unwrap().focused_workspace.as_ref().unwrap() };
        Ok(focused_workspace)
    }

    pub fn workspaces(&self, conn: &mut I3Connection) -> Result<Rc<Workspaces>, MessageError> {
        if self.workspaces.borrow().is_none() {
            self.workspaces
                .borrow_mut()
                .replace(Rc::new(conn.get_workspaces()?));
        }
        Ok(Rc::clone(self.workspaces.borrow().as_ref().unwrap()))
    }

    #[allow(dead_code)]
    pub fn outputs(&self, conn: &mut I3Connection) -> Result<Rc<Outputs>, MessageError> {
        if self.outputs.borrow().is_none() {
            self.outputs
                .borrow_mut()
                .replace(Rc::new(conn.get_outputs()?));
        }
        Ok(Rc::clone(self.outputs.borrow().as_ref().unwrap()))
    }
}
