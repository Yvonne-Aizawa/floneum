use std::fmt::Debug;

use dioxus::{html::geometry::euclid::Point2D, prelude::*};
use petgraph::{
    visit::{EdgeRef, IntoNodeIdentifiers},
    Graph,
};

use crate::{Connection, Edge, LocalSubscription, Node};

pub struct VisualGraphInner {
    pub graph: Graph<LocalSubscription<Node>, LocalSubscription<Edge>>,
    pub currently_dragging: Option<CurrentlyDragging>,
}

#[derive(PartialEq, Clone)]
pub enum CurrentlyDragging {
    Node(NodeDragInfo),
    Connection(CurrentlyDraggingProps),
}

impl Debug for CurrentlyDragging {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CurrentlyDragging::Node(_) => write!(f, "Node"),
            CurrentlyDragging::Connection(_) => write!(f, "Connection"),
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct NodeDragInfo {
    pub element_offset: Point2D<f32, f32>,
    pub node: LocalSubscription<Node>,
}

#[derive(PartialEq, Clone)]
pub enum DraggingIndex {
    Input(usize),
    Output(usize),
}

#[derive(Props, PartialEq, Clone)]
pub struct CurrentlyDraggingProps {
    pub from: LocalSubscription<Node>,
    pub index: DraggingIndex,
    pub to: LocalSubscription<Point2D<f32, f32>>,
}

#[derive(Props, Clone)]
pub struct VisualGraph {
    pub inner: LocalSubscription<VisualGraphInner>,
}

impl VisualGraph {
    pub fn clear_dragging(&self) {
        self.inner.write().currently_dragging = None;
    }

    pub fn update_mouse(&self, evt: &MouseData) {
        let mut inner = self.inner.write();
        match &mut inner.currently_dragging {
            Some(CurrentlyDragging::Connection(current_graph_dragging)) => {
                let mut to = current_graph_dragging.to.write();
                to.x = evt.page_coordinates().x as f32;
                to.y = evt.page_coordinates().y as f32;
            }
            Some(CurrentlyDragging::Node(current_graph_dragging)) => {
                let mut node = current_graph_dragging.node.write();
                node.position.x =
                    evt.page_coordinates().x as f32 - current_graph_dragging.element_offset.x;
                node.position.y =
                    evt.page_coordinates().y as f32 - current_graph_dragging.element_offset.y;
            }
            _ => {}
        }
    }

    pub fn start_dragging_node(&self, evt: &MouseData, node: LocalSubscription<Node>) {
        let mut inner = self.inner.write();
        inner.currently_dragging = Some(CurrentlyDragging::Node(NodeDragInfo {
            node,
            element_offset: Point2D::new(
                evt.element_coordinates().x as f32,
                evt.element_coordinates().y as f32,
            ),
        }));
    }
}

impl PartialEq for VisualGraph {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

#[derive(Props, PartialEq)]
pub struct FlowViewProps {
    graph: VisualGraph,
}

pub fn FlowView(cx: Scope<FlowViewProps>) -> Element {
    use_context_provider(cx, || cx.props.graph.clone());
    let graph = cx.props.graph.inner.use_(cx);
    let current_graph = graph.read();
    let current_graph_dragging = current_graph.currently_dragging.clone();

    render! {
        div { position: "relative",
            svg {
                width: "100%",
                height: "100%",
                onmouseenter: move |data| {
                    if data.held_buttons().is_empty() {
                        cx.props.graph.clear_dragging();
                    }
                },
                onmouseup: move |_| {
                    cx.props.graph.clear_dragging();
                },
                onmousemove: move |evt| {
                    cx.props.graph.update_mouse(&**evt);
                },

                current_graph.graph.edge_references().map(|edge_ref|{
                    let edge = current_graph.graph[edge_ref.id()].clone();
                    let start_id = edge_ref.target();
                    let start = current_graph.graph[start_id].clone();
                    let end_id = edge_ref.source();
                    let end = current_graph.graph[end_id].clone();
                    rsx! {
                        NodeConnection {
                            start: start,
                            connection: edge,
                            end: end,
                        }
                    }
                }),
                current_graph.graph.node_identifiers().map(|node|{
                    let node = current_graph.graph[node].clone();
                    rsx! {
                        Node {
                            node: node,
                        }
                    }
                }),

                if let Some(CurrentlyDragging::Connection(current_graph_dragging)) = &current_graph_dragging {
                    let current_graph_dragging = current_graph_dragging.clone();
                    rsx! {
                        CurrentlyDragging {
                            from: current_graph_dragging.from,
                            index: current_graph_dragging.index,
                            to: current_graph_dragging.to,
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq)]
struct ConnectionProps {
    start: LocalSubscription<Node>,
    connection: LocalSubscription<Edge>,
    end: LocalSubscription<Node>,
}

fn CurrentlyDragging(cx: Scope<CurrentlyDraggingProps>) -> Element {
    let start = cx.props.from.use_(cx);
    let start_pos = match cx.props.index {
        DraggingIndex::Input(index) => start.read().input_pos(index),
        DraggingIndex::Output(index) => start.read().output_pos(index),
    };
    let end = cx.props.to.use_(cx);
    let end_pos = end.read();

    render! { Connection { start_pos: start_pos, end_pos: *end_pos } }
}

fn NodeConnection(cx: Scope<ConnectionProps>) -> Element {
    let start = cx.props.start.use_(cx);
    let connection = cx.props.connection.use_(cx);
    let end = cx.props.end.use_(cx);

    let current_connection = connection.read();
    let start_index = current_connection.start;
    let start = start.read().input_pos(start_index);
    let end_index = current_connection.end;
    let end = end.read().output_pos(end_index);

    render! { Connection { start_pos: start, end_pos: end } }
}
