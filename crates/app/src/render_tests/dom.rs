//! A small DOM for the render tests: it follows the edits the virtual DOM
//! sends (`WriteMutations`), the way the webview's interpreter does, so a
//! test can find a button by its words and tap it.

use std::collections::HashMap;

use dioxus::dioxus_core::{
    AttributeValue, ElementId, Template, TemplateAttribute, TemplateNode, WriteMutations,
};

#[derive(Default, Debug)]
pub struct Node {
    /// None for a text node.
    pub tag: Option<&'static str>,
    pub text: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
    pub listeners: Vec<&'static str>,
    pub id: Option<usize>,
}

pub struct Dom {
    pub nodes: Vec<Node>,
    by_id: HashMap<usize, usize>,
    stack: Vec<usize>,
}

impl Default for Dom {
    fn default() -> Self {
        let root = Node { tag: Some("#root"), id: Some(0), ..Default::default() };
        Dom { nodes: vec![root], by_id: HashMap::from([(0, 0)]), stack: Vec::new() }
    }
}

impl Dom {
    fn add(&mut self, node: Node) -> usize {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    fn build(&mut self, t: &TemplateNode) -> usize {
        match t {
            TemplateNode::Element { tag, attrs, children, .. } => {
                let mut node = Node { tag: Some(tag), ..Default::default() };
                for a in attrs.iter() {
                    if let TemplateAttribute::Static { name, value, .. } = a {
                        node.attrs.push((name.to_string(), value.to_string()));
                    }
                }
                let me = self.add(node);
                for c in children.iter() {
                    let k = self.build(c);
                    self.nodes[k].parent = Some(me);
                    self.nodes[me].children.push(k);
                }
                me
            }
            TemplateNode::Text { text } => self.add(Node { text: text.to_string(), ..Default::default() }),
            TemplateNode::Dynamic { .. } => {
                self.add(Node { tag: Some("#placeholder"), ..Default::default() })
            }
        }
    }

    fn at_path(&self, path: &[u8]) -> usize {
        let mut n = *self.stack.last().expect("a node on the stack");
        for &i in path {
            n = self.nodes[n].children[i as usize];
        }
        n
    }

    fn node(&self, id: ElementId) -> usize {
        *self.by_id.get(&id.0).unwrap_or_else(|| panic!("no node {id:?}"))
    }

    fn assign(&mut self, n: usize, id: ElementId) {
        self.nodes[n].id = Some(id.0);
        self.by_id.insert(id.0, n);
    }

    fn detach(&mut self, n: usize) {
        if let Some(p) = self.nodes[n].parent.take() {
            self.nodes[p].children.retain(|&c| c != n);
        }
    }

    fn pop(&mut self, m: usize) -> Vec<usize> {
        let at = self.stack.len() - m;
        self.stack.split_off(at)
    }

    /// Puts `new` where `old` is, in its parent.
    fn put(&mut self, old: usize, new: Vec<usize>, after: bool, replace: bool) {
        for &n in &new {
            self.detach(n);
        }
        let Some(p) = self.nodes[old].parent else { return };
        let mut at = self.nodes[p].children.iter().position(|&c| c == old).unwrap();
        if after {
            at += 1;
        }
        if replace {
            self.nodes[p].children.remove(at);
            self.nodes[old].parent = None;
        }
        for (k, &n) in new.iter().enumerate() {
            self.nodes[n].parent = Some(p);
            self.nodes[p].children.insert(at + k, n);
        }
    }

    /// Whether the node is still in the page.
    pub fn attached(&self, mut n: usize) -> bool {
        loop {
            if n == 0 {
                return true;
            }
            match self.nodes[n].parent {
                Some(p) => n = p,
                None => return false,
            }
        }
    }

    pub fn text(&self, n: usize) -> String {
        let mut out = String::new();
        self.text_into(n, &mut out);
        out
    }

    fn text_into(&self, n: usize, out: &mut String) {
        let node = &self.nodes[n];
        if matches!(node.tag, Some("style" | "script")) {
            return;
        }
        if node.tag.is_none() {
            out.push_str(&node.text);
        }
        for &c in &node.children {
            self.text_into(c, out);
        }
    }

    pub fn attr(&self, n: usize, name: &str) -> Option<&str> {
        self.nodes[n].attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// Every node in the page, in document order.
    pub fn walk(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut todo = vec![0];
        while let Some(n) = todo.pop() {
            out.push(n);
            todo.extend(self.nodes[n].children.iter().rev());
        }
        out
    }
}

impl WriteMutations for Dom {
    fn append_children(&mut self, id: ElementId, m: usize) {
        let parent = self.node(id);
        for n in self.pop(m) {
            self.detach(n);
            self.nodes[n].parent = Some(parent);
            self.nodes[parent].children.push(n);
        }
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        let n = self.at_path(path);
        self.assign(n, id);
    }

    fn create_placeholder(&mut self, id: ElementId) {
        let n = self.add(Node { tag: Some("#placeholder"), ..Default::default() });
        self.assign(n, id);
        self.stack.push(n);
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        let n = self.add(Node { text: value.to_string(), ..Default::default() });
        self.assign(n, id);
        self.stack.push(n);
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        let n = self.build(&template.roots[index]);
        self.assign(n, id);
        self.stack.push(n);
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        let old = self.node(id);
        let new = self.pop(m);
        self.put(old, new, false, true);
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        let new = self.pop(m);
        let old = self.at_path(path);
        self.put(old, new, false, true);
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        let old = self.node(id);
        let new = self.pop(m);
        self.put(old, new, true, false);
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        let old = self.node(id);
        let new = self.pop(m);
        self.put(old, new, false, false);
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        _ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        let n = self.node(id);
        let attrs = &mut self.nodes[n].attrs;
        attrs.retain(|(k, _)| k != name);
        let v = match value {
            AttributeValue::Text(s) => s.clone(),
            AttributeValue::Float(f) => f.to_string(),
            AttributeValue::Int(i) => i.to_string(),
            AttributeValue::Bool(b) => b.to_string(),
            AttributeValue::None => return,
            _ => return,
        };
        attrs.push((name.to_string(), v));
    }

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        let n = self.node(id);
        self.nodes[n].text = value.to_string();
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        let n = self.node(id);
        if !self.nodes[n].listeners.contains(&name) {
            self.nodes[n].listeners.push(name);
        }
    }

    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        let n = self.node(id);
        self.nodes[n].listeners.retain(|l| *l != name);
    }

    fn remove_node(&mut self, id: ElementId) {
        let n = self.node(id);
        self.detach(n);
    }

    fn push_root(&mut self, id: ElementId) {
        let n = self.node(id);
        self.stack.push(n);
    }
}
