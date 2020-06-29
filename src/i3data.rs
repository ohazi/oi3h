use i3ipc::reply::{Node, Outputs, Workspaces};
use i3ipc::I3Connection;
use i3ipc::MessageError;

use std::marker::PhantomPinned;
use std::pin::Pin;
use std::ptr::NonNull;

use std::cell::Cell;
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
    nodes: Cell<Option<Pin<Box<I3Nodes>>>>,
    workspaces: RefCell<Option<Rc<Workspaces>>>,
    outputs: RefCell<Option<Rc<Outputs>>>,
}

impl I3Data {
    pub fn empty() -> I3Data {
        I3Data {
            nodes: Cell::new(None),
            workspaces: RefCell::new(None),
            outputs: RefCell::new(None),
        }
    }

    pub fn tree(&self, conn: &mut I3Connection) -> Result<&Node, MessageError> {
        if unsafe { (*self.nodes.as_ptr()).is_none() } {
            self.nodes.set(Some(Box::pin(I3Nodes {
                full_tree: conn.get_tree()?,
                focused_node: None,
                focused_workspace: None,
                _pin: PhantomPinned,
            })));
        }
        let pin_box: &Pin<Box<I3Nodes>> = unsafe { (*self.nodes.as_ptr()).as_ref().unwrap() };
        let pin_ref: Pin<&I3Nodes> = Pin::as_ref(pin_box);
        let tree: &Node = &pin_ref.get_ref().full_tree;
        Ok(tree)
    }

    pub fn focused_node(&self, conn: &mut I3Connection) -> Result<&Node, String> {
        if unsafe { (*self.nodes.as_ptr()).is_none() } {
            self.tree(conn).map_err(|e| format!("{}", e))?;
        }
        if unsafe {
            (*self.nodes.as_ptr())
                .as_ref()
                .unwrap()
                .focused_node
                .is_none()
        } {
            let borrow_mut = unsafe { (*self.nodes.as_ptr()).as_mut().unwrap() };
            let mut_ref = Pin::as_mut(borrow_mut);
            let tree = &mut_ref.full_tree;
            unsafe {
                Pin::get_unchecked_mut(mut_ref).focused_node = Some(NonNull::from(
                    criteria::i3_find_focused_node(tree).ok_or("Unable to find focused node")?,
                ));
            }
        }
        let pin_box: &Pin<Box<I3Nodes>> = unsafe { (*self.nodes.as_ptr()).as_ref().unwrap() };
        let pin_ref: Pin<&I3Nodes> = Pin::as_ref(pin_box);
        let focused_node: &Node =
            unsafe { pin_ref.get_ref().focused_node.as_ref().unwrap().as_ref() };
        Ok(focused_node)
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
