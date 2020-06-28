use i3ipc::reply::{Node, Outputs, Workspaces};
use i3ipc::I3Connection;
use i3ipc::MessageError;

use std::marker::PhantomPinned;
use std::pin::Pin;
use std::ptr::NonNull;

use std::cell::RefCell;
use std::rc::Rc;

use crate::criteria;

struct I3Nodes {
    full_tree: Node,
    focused_node: Option<NonNull<Node>>,
    focused_workspace: Option<NonNull<Node>>,
    _pin: PhantomPinned,
}

pub struct I3Data {
    nodes: Option<Pin<Box<I3Nodes>>>,
    workspaces: RefCell<Option<Rc<Workspaces>>>,
    outputs: RefCell<Option<Rc<Outputs>>>,
}

impl I3Data {
    pub fn empty() -> I3Data {
        I3Data {
            nodes: None,
            workspaces: RefCell::new(None),
            outputs: RefCell::new(None),
        }
    }

    pub fn get_tree(&mut self, conn: &mut I3Connection) -> Result<&Node, String> {
        if self.nodes.is_none() {
            self.nodes = Some(Box::pin(I3Nodes {
                full_tree: conn.get_tree().map_err(|e| format!("{}", e))?,
                focused_node: None,
                focused_workspace: None,
                _pin: PhantomPinned,
            }));
        }
        Ok(&self.nodes.as_ref().unwrap().as_ref().get_ref().full_tree)
    }

    pub fn tree(&self) -> Option<&Node> {
        self.nodes.as_ref().map(|n| &n.as_ref().get_ref().full_tree)
    }

    pub fn get_focused_node(&mut self, conn: &mut I3Connection) -> Result<&Node, String> {
        if self.nodes.is_none() {
            self.get_tree(conn)?;
        }
        if self.nodes.as_ref().unwrap().focused_node.is_none() {
            let mut_ref = Pin::as_mut(self.nodes.as_mut().unwrap());
            let tree = &mut_ref.full_tree;
            unsafe {
                Pin::get_unchecked_mut(mut_ref).focused_node = Some(NonNull::from(
                    criteria::i3_find_focused_node(tree).ok_or("Unable to find focused node")?,
                ));
            }
        }
        unsafe {
            Ok(self
                .nodes
                .as_ref()
                .unwrap()
                .focused_node
                .as_ref()
                .unwrap()
                .as_ref())
        }
    }

    pub fn focused_node(&self) -> Option<&Node> {
        unsafe {
            Some(
                self.nodes
                    .as_ref()
                    .unwrap()
                    .focused_node
                    .as_ref()
                    .unwrap()
                    .as_ref(),
            )
        }
    }

    #[allow(dead_code)]
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
