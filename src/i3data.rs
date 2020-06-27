use i3ipc::reply::{Node, Outputs, Workspaces};
use i3ipc::I3Connection;

use std::marker::PhantomPinned;
use std::pin::Pin;
use std::ptr::NonNull;

use crate::criteria;

#[derive(Debug)]
pub struct I3Data {
    tree: Option<Node>,
    outputs: Option<Outputs>,
    workspaces: Option<Workspaces>,
    focused_node: Option<NonNull<Node>>,
    _pin: PhantomPinned,
}

impl I3Data {
    pub fn empty() -> Pin<Box<I3Data>> {
        let data = I3Data {
            tree: None,
            outputs: None,
            workspaces: None,
            focused_node: None,
            _pin: PhantomPinned,
        };

        Box::pin(data)
    }

    pub fn get_tree<'a>(
        mut self: Pin<&'a mut Self>,
        conn: &mut I3Connection,
    ) -> Result<&'a Node, String> {
        if self.tree.is_none() {
            unsafe {
                let mut_ref = Pin::as_mut(&mut self);
                Pin::get_unchecked_mut(mut_ref).tree =
                    Some(conn.get_tree().map_err(|e| format!("{}", e))?);
            }
        }
        Ok(self.into_ref().get_ref().tree.as_ref().unwrap())
    }

    pub fn tree<'a>(self: Pin<&'a Self>) -> Option<&'a Node> {
        self.get_ref().tree.as_ref()
    }

    #[allow(dead_code)]
    pub fn get_outputs<'a>(
        mut self: Pin<&'a mut Self>,
        conn: &mut I3Connection,
    ) -> Result<&'a Outputs, String> {
        if self.outputs.is_none() {
            unsafe {
                let mut_ref = Pin::as_mut(&mut self);
                Pin::get_unchecked_mut(mut_ref).outputs =
                    Some(conn.get_outputs().map_err(|e| format!("{}", e))?);
            }
        }
        Ok(self.into_ref().get_ref().outputs.as_ref().unwrap())
    }

    #[allow(dead_code)]
    pub fn outputs(self: Pin<&Self>) -> Option<&Outputs> {
        self.get_ref().outputs.as_ref()
    }

    pub fn get_workspaces<'a>(
        mut self: Pin<&'a mut Self>,
        conn: &mut I3Connection,
    ) -> Result<&'a Workspaces, String> {
        if self.workspaces.is_none() {
            unsafe {
                let mut_ref = Pin::as_mut(&mut self);
                Pin::get_unchecked_mut(mut_ref).workspaces =
                    Some(conn.get_workspaces().map_err(|e| format!("{}", e))?);
            }
        }
        Ok(Pin::into_ref(self).get_ref().workspaces.as_ref().unwrap())
    }

    pub fn workspaces<'a>(self: Pin<&'a Self>) -> Option<&'a Workspaces> {
        self.get_ref().workspaces.as_ref()
    }

    pub fn get_focused_node<'a>(
        mut self: Pin<&'a mut Self>,
        conn: &mut I3Connection,
    ) -> Result<&'a Node, String> {
        if self.focused_node.is_none() {
            self.as_mut().get_tree(conn)?;
            let mut_ref = Pin::as_mut(&mut self);
            let tree = mut_ref.tree.as_ref().unwrap();
            unsafe {
                Pin::get_unchecked_mut(mut_ref).focused_node = Some(NonNull::from(
                    criteria::i3_find_focused_node(tree).ok_or("Unable to find focused node")?,
                ));
            }
        }
        unsafe {
            Ok(self
                .into_ref()
                .get_ref()
                .focused_node
                .as_ref()
                .unwrap()
                .as_ref())
        }
    }

    pub fn focused_node(self: Pin<&Self>) -> Option<&Node> {
        if self.focused_node.is_none() {
            None
        } else {
            unsafe {
                Some(NonNull::as_ref(
                    self.get_ref().focused_node.as_ref().unwrap(),
                ))
            }
        }
    }
}
