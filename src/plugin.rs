use bevy::prelude::*;
use bevy::render::pass::*;
use bevy::render::pipeline::PipelineDescriptor;
use bevy::render::render_graph::*;
use pixel_widgets::Model;

use crate::event::update_ui;
use crate::pipeline::{build_ui_pipeline, UI_PIPELINE_HANDLE};
use crate::pixel_widgets_node::UiNode;
use crate::UiPlugin;
use crate::style::{Stylesheet, StylesheetLoader};

const PIXEL_WIDGETS: &'static str = "pixel_widgets";

impl<M: Model + Send + Sync> Plugin for UiPlugin<M> {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(update_ui::<M>.system());
        app.add_asset::<Stylesheet>();
        app.init_asset_loader::<StylesheetLoader>();

        let resources = app.resources();

        let msaa = resources.get::<Msaa>().unwrap();
        let pass_descriptor = PassDescriptor {
            color_attachments: vec![msaa.color_attachment_descriptor(
                TextureAttachment::Input("color_attachment".to_string()),
                TextureAttachment::Input("color_resolve_target".to_string()),
                Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            )],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
                attachment: TextureAttachment::Input("depth".to_string()),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
            sample_count: msaa.samples,
        };

        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_system_node(PIXEL_WIDGETS, UiNode::<M>::new(pass_descriptor));
        render_graph
            .add_slot_edge(
                base::node::PRIMARY_SWAP_CHAIN,
                WindowSwapChainNode::OUT_TEXTURE,
                PIXEL_WIDGETS,
                if msaa.samples > 1 {
                    "color_resolve_target"
                } else {
                    "color_attachment"
                },
            )
            .unwrap();

        render_graph
            .add_slot_edge(
                base::node::MAIN_DEPTH_TEXTURE,
                WindowTextureNode::OUT_TEXTURE,
                PIXEL_WIDGETS,
                "depth",
            )
            .unwrap();

        if msaa.samples > 1 {
            render_graph
                .add_slot_edge(
                    base::node::MAIN_SAMPLED_COLOR_ATTACHMENT,
                    WindowSwapChainNode::OUT_TEXTURE,
                    PIXEL_WIDGETS,
                    "color_attachment",
                )
                .unwrap();
        }
        render_graph
            .add_node_edge(base::node::MAIN_PASS, PIXEL_WIDGETS)
            .unwrap();

        let mut pipelines = resources.get_mut::<Assets<PipelineDescriptor>>().unwrap();
        let mut shaders = resources.get_mut::<Assets<Shader>>().unwrap();
        pipelines.set_untracked(UI_PIPELINE_HANDLE, build_ui_pipeline(&mut shaders));
    }
}

impl<M: Model + Send + Sync> Default for UiPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}
