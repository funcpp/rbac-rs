use eframe::{run_native, App, CreationContext};
use egui::{CentralPanel, Context};
use egui_graphs::{DefaultEdgeShape, Graph, GraphView, SettingsInteraction, SettingsNavigation};
use node::NodeShapeFlex;
use petgraph::{
    stable_graph::{DefaultIx, EdgeIndex, NodeIndex, StableGraph},
    Directed,
};

use std::collections::HashMap;
mod node;

pub struct FlexNodesApp {
    g: Graph<node::NodeData, (), Directed, DefaultIx, NodeShapeFlex, DefaultEdgeShape>,
    selected_node: Option<NodeIndex>,
    selected_edge: Option<EdgeIndex>,
}

impl FlexNodesApp {
    fn new(_: &CreationContext<'_>) -> Self {
        let g = generate_graph();
        Self {
            g: Graph::from(&g),
            selected_node: Option::default(),
            selected_edge: Option::default(),
        }
    }

    fn read_data(&mut self) {
        if !self.g.selected_nodes().is_empty() {
            let idx = self.g.selected_nodes().first().unwrap();
            self.selected_node = Some(*idx);
            self.selected_edge = None;
        }
        if !self.g.selected_edges().is_empty() {
            let idx = self.g.selected_edges().first().unwrap();
            self.selected_edge = Some(*idx);
            self.selected_node = None;
        }
    }

    fn render(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            let widget =
                &mut GraphView::<_, _, _, _, NodeShapeFlex, DefaultEdgeShape>::new(&mut self.g)
                    .with_interactions(
                        &SettingsInteraction::default()
                            .with_dragging_enabled(true)
                            .with_node_selection_enabled(true)
                            .with_edge_selection_enabled(true),
                    )
                    .with_navigations(
                        &SettingsNavigation::default()
                            .with_fit_to_screen_enabled(false)
                            .with_zoom_and_pan_enabled(true),
                    );
            ui.add(widget);
        });
    }
}

impl App for FlexNodesApp {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        self.read_data();
        self.render(ctx);
    }
}

fn generate_graph() -> StableGraph<node::NodeData, ()> {
    let mut g = StableGraph::new();

    use rbac::RBAC;

    let rbac = RBAC::new("./rbac-example/rocksdb/test2");

    let nodes = rbac.get_all_vertices().unwrap();
    let edges = rbac.get_all_edges().unwrap();

    let mut node_index_map = HashMap::new();

    for node in nodes {
        let idx = g.add_node(node::NodeData {
            entity: node.t.to_string(),
        });
        node_index_map.insert(node.id, idx);
    }

    for edge in edges {
        let a = node_index_map.get(&edge.outbound_id).unwrap();
        let b = node_index_map.get(&edge.inbound_id).unwrap();
        g.add_edge(*a, *b, ());
    }

    g
}

fn main() {
    let native_options = eframe::NativeOptions::default();
    run_native(
        "rbac-visualize",
        native_options,
        Box::new(|cc| Ok(Box::new(FlexNodesApp::new(cc)))),
    )
    .unwrap();
}
