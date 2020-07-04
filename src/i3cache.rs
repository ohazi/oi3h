//! A cache for i3 IPC output and tree search operations that may be expensive to repeat.

use i3ipc::reply::{Node, Outputs, Workspaces};
use i3ipc::I3Connection;
use i3ipc::MessageError;

use std::marker::PhantomPinned;
use std::pin::Pin;

use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use crate::search;

/// Holds the root `Node` of the i3 tree, as well as any references to other `Node`s inside this
/// tree.
struct I3Nodes {
    full_tree: Node,
    focused_node: *const Node,
    focused_workspace: *const Node,
    _pin: PhantomPinned,
}

/// Caches output from the i3 IPC channel, as well as results of search operations that may be
/// expensive to repeat.
pub struct I3Cache {
    nodes: Cell<Option<Pin<Box<I3Nodes>>>>,
    workspaces: RefCell<Option<Rc<Workspaces>>>,
    outputs: RefCell<Option<Rc<Outputs>>>,
}

impl I3Cache {
    pub fn new() -> I3Cache {
        I3Cache {
            nodes: Cell::new(None),
            workspaces: RefCell::new(None),
            outputs: RefCell::new(None),
        }
    }

    /// Returns a shared reference to the nodes field, if it exists.
    ///
    /// # Safety
    /// The only unsafe operation performed by this function is dereferencing `self.nodes.as_ptr()`,
    /// which cannot fail. Furthermore, `nodes` is pinned as soon as it is set, so it is safe to hold
    /// onto shared references to `nodes` as long as they don't outlive `self`. Other references
    /// within nodes (such as `focused_node`) may change while the shared reference to `nodes` is
    /// held, however, they are raw pointers, so dereferencing them is unsafe anyway.
    fn nodes(&self) -> Option<&I3Nodes> {
        let orpb_i3n: Option<&Pin<Box<I3Nodes>>> = unsafe { (*self.nodes.as_ptr()).as_ref() };
        let or_i3n: Option<&I3Nodes> = orpb_i3n.map(|i| i.as_ref().get_ref());
        or_i3n
    }

    /// Returns a pinned, mutable reference to the nodes field, if it exists.
    ///
    /// # Safety
    /// As with `nodes()`, the only unsafe operation performed by this function is dereferencing
    /// `self.nodes.as_ptr()`, which cannot fail. In order to do anything unsafe with the output of
    /// this function, you would need to use unsafe code, such as:
    /// ```
    /// let nodes_mut = self.nodes_mut().unwrap();
    /// unsafe {
    ///     Pin::get_unchecked_mut(nodes_mut).focused_node = ...
    /// }
    /// ```
    /// Callers of this function should be aware of any existing shared references to `nodes` or
    /// subfields when performing any unsafe operations with the returned value.
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
                    search::i3_find_focused_node(full_tree).ok_or("Unable to find focused node")?
                        as *const Node;
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
                    search::i3_find_focused_workspace(&workspaces, tree)
                        .ok_or("Unable to find focused workspace")?
                        as *const Node;
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
