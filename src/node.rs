use dioxus::{html::geometry::euclid::Point2D, prelude::*};
use floneum_plugin::exports::plugins::main::definitions::{Input, Output};
use floneum_plugin::PluginInstance;
use petgraph::{graph::NodeIndex, stable_graph::DefaultIx};
use serde::{Deserialize, Serialize};

use crate::graph::CurrentlyDragging;
use crate::{local_sub::LocalSubscription, Point, VisualGraph};
use crate::{use_application_state, CurrentlyDraggingProps, DraggingIndex, Edge};

const SNAP_DISTANCE: f32 = 15.;

#[derive(Serialize, Deserialize)]
pub struct Node {
    pub instance: PluginInstance,
    #[serde(skip)]
    pub running: bool,
    #[serde(skip)]
    pub queued: bool,
    #[serde(skip)]
    pub error: Option<String>,
    pub id: NodeIndex<DefaultIx>,
    pub position: Point,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub width: f32,
    pub height: f32,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Node {
    pub fn center(&self) -> Point2D<f32, f32> {
        (Point2D::new(self.position.x, self.position.y)
            - Point2D::new(self.width, self.height) / 2.)
            .to_point()
    }

    pub fn input_pos(&self, index: usize) -> Point2D<f32, f32> {
        Point2D::new(
            self.position.x - 1.,
            self.position.y + ((index as f32 + 1.) * self.height / (self.inputs.len() as f32 + 1.)),
        )
    }

    pub fn output_pos(&self, index: usize) -> Point2D<f32, f32> {
        Point2D::new(
            self.position.x + self.width - 1.,
            self.position.y
                + ((index as f32 + 1.) * self.height / (self.outputs.len() as f32 + 1.)),
        )
    }

    pub fn help_text(&self) -> String {
        self.instance.metadata().description.to_string()
    }
}

#[derive(Props, PartialEq)]
pub struct NodeProps {
    node: LocalSubscription<Node>,
}

pub fn Node(cx: Scope<NodeProps>) -> Element {
    let application = use_application_state(cx).use_(cx);
    let node = cx.props.node.use_(cx);
    let current_node = node.read();
    let current_node_id = current_node.id;
    let width = current_node.width;
    let height = current_node.height;
    let pos = current_node.position;
    let node_size = 5.;

    if current_node.running {
        return render! { div { "Loading..." } };
    }

    render! {
        // inputs
        (0..current_node.inputs.len()).map(|i| {
            let pos = current_node.input_pos(i);
            rsx! {
                circle {
                    cx: pos.x as f64 + node_size,
                    cy: pos.y as f64,
                    r: node_size,
                    onmousedown: move |evt| {
                        let graph: VisualGraph = cx.consume_context().unwrap();
                        graph.inner.write().currently_dragging = Some(CurrentlyDragging::Connection(CurrentlyDraggingProps {
                            from: cx.props.node.clone(),
                            index: DraggingIndex::Input(i),
                            to: LocalSubscription::new(Point2D::new(evt.page_coordinates().x as f32, evt.page_coordinates().y as f32)),
                        }));
                    },
                    onmouseup: move |_| {
                        // Set this as the end of the connection if we're currently dragging and this is the right type of connection
                        let graph: VisualGraph = cx.consume_context().unwrap();
                        let mut current_graph = graph.inner.write();
                        if let Some(CurrentlyDragging::Connection(currently_dragging)) = &current_graph.currently_dragging {
                            let start_index = match currently_dragging.index {
                                DraggingIndex::Output(index) => index,
                                _ => return,
                            };
                            let start_id = currently_dragging.from.read(cx).id;
                            let edge = LocalSubscription::new(Edge::new(
                                start_index,
                                i,
                            ));
                            current_graph.graph.add_edge(start_id, current_node_id, edge);
                        }
                        graph.clear_dragging();
                    },
                    onmousemove: move |evt| {
                        let graph: VisualGraph = cx.consume_context().unwrap();
                        graph.update_mouse(&**evt);
                    },
                }
            }
        }),

        // center UI/Configuration
        foreignObject {
            x: "{pos.x}",
            y: "{pos.y}",
            width: width as f64,
            height: height as f64,
            onmousedown: move |evt| {
                let graph: VisualGraph = cx.consume_context().unwrap();
                {
                    let node = node.read();
                    if let Some((index, dist))
                        = (0..node.inputs.len())
                            .map(|i| {
                                let input_pos = node.input_pos(i);
                                (
                                    DraggingIndex::Input(i),
                                    (input_pos.x - evt.page_coordinates().x as f32).powi(2)
                                        + (input_pos.y - evt.page_coordinates().y as f32).powi(2),
                                )
                            })
                            .chain(
                                (0..node.outputs.len())
                                    .map(|i| {
                                        let output_pos = node.output_pos(i);
                                        (
                                            DraggingIndex::Output(i),
                                            (output_pos.x - evt.page_coordinates().x as f32).powi(2)
                                                + (output_pos.y - evt.page_coordinates().y as f32).powi(2),
                                        )
                                    }),
                            )
                            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                    {
                        if dist < SNAP_DISTANCE.powi(2) {
                            let mut current_graph = graph.inner.write();
                            current_graph
                                .currently_dragging = Some(
                                CurrentlyDragging::Connection(CurrentlyDraggingProps {
                                    from: cx.props.node.clone(),
                                    index,
                                    to: LocalSubscription::new(
                                        Point2D::new(
                                            evt.page_coordinates().x as f32,
                                            evt.page_coordinates().y as f32,
                                        ),
                                    ),
                                }),
                            );
                        } else {
                            graph.start_dragging_node(&*evt, cx.props.node.clone());
                        }
                    } else {
                        graph.start_dragging_node(&*evt, cx.props.node.clone());
                    }
                }
            },
            onmousemove: |evt| {
                let graph: VisualGraph = cx.consume_context().unwrap();
                graph.update_mouse(&**evt);
            },
            onmouseup: move |evt| {
                let graph: VisualGraph = cx.consume_context().unwrap();
                {
                    let mut current_graph = graph.inner.write();
                    if let Some(CurrentlyDragging::Connection(currently_dragging))
                        = &current_graph.currently_dragging
                    {
                        let dist;
                        let edge;
                        let start_id;
                        let end_id;
                        match currently_dragging.index {
                            DraggingIndex::Output(start_index) => {
                                let node = node.read();
                                let combined = (0..node.inputs.len())
                                    .map(|i| {
                                        let input_pos = node.input_pos(i);
                                        (
                                            i,
                                            (input_pos.x - evt.page_coordinates().x as f32).powi(2)
                                                + (input_pos.y - evt.page_coordinates().y as f32).powi(2),
                                        )
                                    })
                                    .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                                    .unwrap();
                                let input_idx = combined.0;
                                dist = combined.1;
                                start_id = currently_dragging.from.read(cx).id;
                                end_id = current_node_id;
                                edge = LocalSubscription::new(Edge::new(start_index, input_idx));
                            }
                            DraggingIndex::Input(start_index) => {
                                let node = node.read();
                                let combined = (0..node.outputs.len())
                                    .map(|i| {
                                        let output_pos = node.output_pos(i);
                                        (
                                            i,
                                            (output_pos.x - evt.page_coordinates().x as f32).powi(2)
                                                + (output_pos.y - evt.page_coordinates().y as f32).powi(2),
                                        )
                                    })
                                    .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                                    .unwrap();
                                let output_idx = combined.0;
                                dist = combined.1;
                                end_id = currently_dragging.from.read(cx).id;
                                start_id = current_node_id;
                                edge = LocalSubscription::new(Edge::new(output_idx, start_index));
                            }
                        }
                        if dist < SNAP_DISTANCE.powi(2) {
                            current_graph.graph.add_edge(start_id, end_id, edge);
                        }
                    }
                }
                graph.clear_dragging();

                // Focus or unfocus this node
                let mut application = application.write();
                match &application.currently_focused {
                    Some(currently_focused_node) if currently_focused_node == &cx.props.node => {
                        application.currently_focused = None;
                    }
                    _ => {
                        application.currently_focused = Some(cx.props.node.clone());
                    }
                }
            },

            CenterNodeUI {
                node: cx.props.node.clone(),
            }
        }

        // outputs
        (0..current_node.outputs.len()).map(|i| {
            let pos = current_node.output_pos(i);
            rsx! {
                circle {
                    cx: pos.x as f64 - node_size,
                    cy: pos.y as f64,
                    r: node_size,
                    onmousedown: move |evt| {
                        let graph: VisualGraph = cx.consume_context().unwrap();
                        graph.inner.write().currently_dragging = Some(CurrentlyDragging::Connection(CurrentlyDraggingProps {
                            from: cx.props.node.clone(),
                            index: DraggingIndex::Output(i),
                            to: LocalSubscription::new(Point2D::new(evt.page_coordinates().x as f32, evt.page_coordinates().y as f32)),
                        }));
                    },
                    onmouseup: move |_| {
                        // Set this as the end of the connection if we're currently dragging and this is the right type of connection
                        let graph: VisualGraph = cx.consume_context().unwrap();
                        {
                            let mut current_graph = graph.inner.write();
                            if let Some(CurrentlyDragging::Connection(currently_dragging)) = &current_graph.currently_dragging {
                                let start_index = match currently_dragging.index {
                                    DraggingIndex::Input(index) => index,
                                    _ => return,
                                };
                                let start_id = currently_dragging.from.read(cx).id;
                                let edge = LocalSubscription::new(Edge::new(i, start_index));
                                current_graph.graph.add_edge(current_node_id, start_id, edge);
                            }
                        }
                        graph.clear_dragging();
                    },
                    onmousemove: move |evt| {
                        let graph: VisualGraph = cx.consume_context().unwrap();
                        graph.update_mouse(&**evt);
                    },
                }
            }
        })
    }
}

fn CenterNodeUI(cx: Scope<NodeProps>) -> Element {
    let application = use_application_state(cx).use_(cx);
    let focused = &application.read().currently_focused == &Some(cx.props.node.clone());
    let node = cx.props.node.use_(cx);
    let current_node = node.read();
    let name = &current_node.instance.metadata().name;
    let node_size = 5.;
    let focused_class = if focused {
        "border-2 border-blue-500"
    } else {
        ""
    };

    render! {
        div {
            style: "-webkit-user-select: none; -ms-user-select: none; user-select: none;",
            class: "flex flex-col justify-center items-center w-full h-full border rounded-md {focused_class}",
            div { padding: "{node_size*2.}px",
                p {
                    "{name}"
                }
                div { color: "red",
                    if let Some(error) = &current_node.error {
                        rsx! {
                            p { "{error}" }
                        }
                    }
                }
            }
        }
    }
}
