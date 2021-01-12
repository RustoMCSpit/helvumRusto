use crate::{view, PipewireLink};

use pipewire::{
    port::Direction,
    registry::{GlobalObject, ObjectType},
};

use std::{cell::RefCell, collections::HashMap, rc::Rc};

enum Item {
    Node(view::Node),
    Port { node_id: u32 },
    Link,
}

/// This struct stores the state of the pipewire graph.
///
/// It receives updates from the [`PipewireConnection`](crate::pipewire_connection::PipewireConnection)
/// responsible for updating it and applies them to its internal state.
///
/// It also keeps the view updated to always reflect this internal state.
pub struct PipewireState {
    graphview: Rc<RefCell<view::GraphView>>,
    items: HashMap<u32, Item>,
}

impl PipewireState {
    pub fn new(graphview: Rc<RefCell<view::GraphView>>) -> Self {
        let result = Self {
            graphview,
            items: HashMap::new(),
        };

        result
    }

    /// This function is called from the `PipewireConnection` struct responsible for updating this struct.
    pub fn global(&mut self, global: GlobalObject) {
        match global.type_ {
            ObjectType::Node => {
                self.add_node(global);
            }
            ObjectType::Port => {
                self.add_port(global);
            }
            ObjectType::Link => {
                self.add_link(global);
            }
            _ => {}
        }
    }

    fn add_node(&mut self, node: GlobalObject) {
        // Update graph to contain the new node.
        let node_widget = crate::view::Node::new(&format!(
            "{}",
            node.props
                .map(|dict| String::from(
                    dict.get("node.nick")
                        .or(dict.get("node.description"))
                        .or(dict.get("node.name"))
                        .unwrap_or_default()
                ))
                .unwrap_or_default()
        ));
        self.graphview
            .borrow_mut()
            .add_node(node.id, node_widget.clone());

        // Save the created widget so we can delete ports easier.
        self.items.insert(node.id, Item::Node(node_widget));
    }

    fn add_port(&mut self, port: GlobalObject) {
        // Update graph to contain the new port.
        let props = port.props.expect("Port object is missing properties");
        let port_label = format!("{}", props.get("port.name").unwrap_or_default());
        let node_id: u32 = props
            .get("node.id")
            .expect("Port has no node.id property!")
            .parse()
            .expect("Could not parse node.id property");
        let new_port = crate::view::port::Port::new(
            port.id,
            &port_label,
            if matches!(props.get("port.direction"), Some("in")) {
                Direction::Input
            } else {
                Direction::Output
            },
        );

        self.graphview
            .borrow_mut()
            .add_port_to_node(node_id, new_port.id, new_port);

        // Save node_id so we can delete this port easily.
        self.items.insert(port.id, Item::Port { node_id });
    }

    fn add_link(&mut self, link: GlobalObject) {
        self.items.insert(link.id, Item::Link);

        // Update graph to contain the new link.
        let props = link.props.expect("Link object is missing properties");
        let input_node: u32 = props
            .get("link.input.node")
            .expect("Link has no link.input.node property")
            .parse()
            .expect("Could not parse link.input.node property");
        let input_port: u32 = props
            .get("link.input.port")
            .expect("Link has no link.input.port property")
            .parse()
            .expect("Could not parse link.input.port property");
        let output_node: u32 = props
            .get("link.output.node")
            .expect("Link has no link.input.node property")
            .parse()
            .expect("Could not parse link.input.node property");
        let output_port: u32 = props
            .get("link.output.port")
            .expect("Link has no link.output.port property")
            .parse()
            .expect("Could not parse link.output.port property");
        self.graphview.borrow_mut().add_link(
            link.id,
            PipewireLink {
                node_from: output_node,
                port_from: output_port,
                node_to: input_node,
                port_to: input_port,
            },
        );
    }

    /// This function is called from the `PipewireConnection` struct responsible for updating this struct.
    pub fn global_remove(&mut self, id: u32) {
        if let Some(item) = self.items.get(&id) {
            match item {
                Item::Node(_) => self.remove_node(id),
                Item::Port { node_id } => self.remove_port(id, *node_id),
                Item::Link => self.remove_link(id),
            }

            self.items.remove(&id);
        } else {
            log::warn!(
                "Attempted to remove item with id {} that is not saved in state",
                id
            );
        }
    }

    fn remove_node(&self, id: u32) {
        self.graphview.borrow().remove_node(id);
    }

    fn remove_port(&self, id: u32, node_id: u32) {
        if let Some(Item::Node(node)) = self.items.get(&node_id) {
            node.remove_port(id);
        }
    }

    fn remove_link(&self, id: u32) {
        self.graphview.borrow().remove_link(id);
    }
}
