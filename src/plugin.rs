use bevy::prelude::*;
use bevy::render::pipeline::PipelineDescriptor;
use bevy::render::render_graph::base::node::MAIN_PASS;
use bevy::render::render_graph::RenderGraph;
use pixel_widgets::Model;

use crate::node::UiNode;
use crate::pipeline::{build_ui_pipeline, UI_PIPELINE_HANDLE};
use crate::UiPlugin;
use crate::event::update_ui;

const PIXEL_WIDGETS: &'static str = "pixel_widgets";

impl<M: Model + Send + Sync> Plugin for UiPlugin<M> {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(update_ui::<M>.system());

        let resources = app.resources();

        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_system_node(PIXEL_WIDGETS, UiNode::<M>::default());
        render_graph.add_node_edge(PIXEL_WIDGETS, MAIN_PASS).unwrap();

        let mut pipelines = resources.get_mut::<Assets<PipelineDescriptor>>().unwrap();
        let mut shaders = resources.get_mut::<Assets<Shader>>().unwrap();
        pipelines.set(UI_PIPELINE_HANDLE, build_ui_pipeline(&mut shaders));
    }
}

impl<M: Model + Send + Sync> Default for UiPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}
