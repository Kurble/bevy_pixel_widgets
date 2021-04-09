use std::ops::Range;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy::render::pass::*;
use bevy::render::pipeline::*;
use bevy::render::render_graph::{CommandQueue, Node, ResourceSlotInfo, ResourceSlots, SystemNode};
use bevy::render::renderer::RenderContext;

use crate::pipeline::UI_PIPELINE_HANDLE;
use crate::style::Stylesheet;

use super::*;
use bevy::utils::HashMap;

pub struct UiNode {
    command_queue: CommandQueue,
    command_buffer: Arc<Mutex<Vec<RenderCommand>>>,
    descriptor: PassDescriptor,
    inputs: Vec<ResourceSlotInfo>,
    color_attachment_input_indices: Vec<Option<usize>>,
    color_resolve_target_indices: Vec<Option<usize>>,
    depth_stencil_attachment_input_index: Option<usize>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RenderCommand {
    SetPipeline {
        pipeline: Handle<PipelineDescriptor>,
    },
    SetScissorRect {
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    },
    SetVertexBuffer {
        slot: u32,
        buffer: BufferId,
        offset: u64,
    },
    SetBindGroup {
        index: u32,
        bind_group: BindGroupId,
        dynamic_uniform_indices: Option<Arc<[u32]>>,
    },
    Draw {
        vertices: Range<u32>,
        instances: Range<u32>,
    },
}

impl Node for UiNode {
    fn input(&self) -> &[ResourceSlotInfo] {
        &self.inputs
    }

    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn RenderContext,
        input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        self.command_queue.execute(render_context);

        let render_resource_bindings = world.get_resource::<RenderResourceBindings>().unwrap();
        let pipelines = world.get_resource::<Assets<PipelineDescriptor>>().unwrap();

        for (i, color_attachment) in self.descriptor.color_attachments.iter_mut().enumerate() {
            if let Some(input_index) = self.color_attachment_input_indices[i] {
                color_attachment.attachment =
                    TextureAttachment::Id(input.get(input_index).unwrap().get_texture().unwrap());
            }
            if let Some(input_index) = self.color_resolve_target_indices[i] {
                color_attachment.resolve_target = Some(TextureAttachment::Id(
                    input.get(input_index).unwrap().get_texture().unwrap(),
                ));
            }
        }

        if let Some(input_index) = self.depth_stencil_attachment_input_index {
            self.descriptor.depth_stencil_attachment.as_mut().unwrap().attachment =
                TextureAttachment::Id(input.get(input_index).unwrap().get_texture().unwrap());
        }

        render_context.begin_pass(&self.descriptor, &render_resource_bindings, &mut |pass| {
            let mut draw_state = DrawState::default();

            for command in self.command_buffer.lock().unwrap().drain(..) {
                match command {
                    RenderCommand::SetPipeline { pipeline } => {
                        pass.set_pipeline(&pipeline);
                        draw_state.set_pipeline(&pipeline, pipelines.get(&pipeline).unwrap());
                    }
                    RenderCommand::SetScissorRect { x, y, w, h } => {
                        pass.set_scissor_rect(x, y, w, h);
                    }
                    RenderCommand::SetVertexBuffer { slot, buffer, offset } => {
                        pass.set_vertex_buffer(slot, buffer, offset);
                        draw_state.set_vertex_buffer(slot, buffer);
                    }
                    RenderCommand::SetBindGroup {
                        index,
                        bind_group,
                        dynamic_uniform_indices,
                    } => {
                        let pipeline = pipelines.get(draw_state.pipeline.as_ref().unwrap()).unwrap();
                        let layout = pipeline.get_layout().unwrap();
                        let bind_group_descriptor = layout.get_bind_group(index).unwrap();
                        pass.set_bind_group(
                            index,
                            bind_group_descriptor.id,
                            bind_group,
                            dynamic_uniform_indices.as_deref(),
                        );
                        draw_state.set_bind_group(index, bind_group);
                    }
                    RenderCommand::Draw { vertices, instances } => {
                        if draw_state.can_draw() {
                            pass.draw(vertices, instances);
                        } else {
                            println!("Could not draw because the pipeline layout wasn't fully set for pipeline: {:?}", draw_state.pipeline);
                        }
                    },
                }
            }
        });
    }
}

impl SystemNode for UiNode {
    fn get_system(&self) -> Box<dyn System<In = (), Out = ()>> {
        let system = render_ui.system().config(|config| {
            config.0 = Some(State {
                command_queue: self.command_queue.clone(),
                command_buffer: self.command_buffer.clone(),
                sampler_id: None,
            });
        });
        Box::new(system)
    }
}

impl UiNode {
    pub fn new(descriptor: PassDescriptor) -> Self {
        let mut inputs = Vec::new();
        let mut color_attachment_input_indices = Vec::new();
        let mut color_resolve_target_indices = Vec::new();
        for color_attachment in descriptor.color_attachments.iter() {
            if let TextureAttachment::Input(ref name) = color_attachment.attachment {
                color_attachment_input_indices.push(Some(inputs.len()));
                inputs.push(ResourceSlotInfo::new(name.to_string(), RenderResourceType::Texture));
            } else {
                color_attachment_input_indices.push(None);
            }

            if let Some(TextureAttachment::Input(ref name)) = color_attachment.resolve_target {
                color_resolve_target_indices.push(Some(inputs.len()));
                inputs.push(ResourceSlotInfo::new(name.to_string(), RenderResourceType::Texture));
            } else {
                color_resolve_target_indices.push(None);
            }
        }

        let mut depth_stencil_attachment_input_index = None;
        if let Some(ref depth_stencil_attachment) = descriptor.depth_stencil_attachment {
            if let TextureAttachment::Input(ref name) = depth_stencil_attachment.attachment {
                depth_stencil_attachment_input_index = Some(inputs.len());
                inputs.push(ResourceSlotInfo::new(name.to_string(), RenderResourceType::Texture));
            }
        }

        Self {
            command_queue: Default::default(),
            command_buffer: Default::default(),
            descriptor,
            inputs,
            color_attachment_input_indices,
            color_resolve_target_indices,
            depth_stencil_attachment_input_index,
        }
    }
}

#[derive(Default)]
struct State {
    command_queue: CommandQueue,
    command_buffer: Arc<Mutex<Vec<RenderCommand>>>,
    sampler_id: Option<SamplerId>,
}

#[allow(clippy::too_many_arguments)]
fn render_ui(
    mut state: Local<State>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut pipeline_compiler: ResMut<PipelineCompiler>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    mut stylesheets: ResMut<Assets<Stylesheet>>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    windows: Res<Windows>,
    mut query: Query<(&mut UiDraw, &Handle<Stylesheet>)>,
) {
    let window = windows.get_primary().unwrap();

    let mut draw: Vec<RenderCommand> = {
        let mut command_buffer = state.command_buffer.lock().unwrap();
        command_buffer.clear();
        std::mem::replace(&mut command_buffer, Vec::new())
    };

    let sampler_id = *state
        .sampler_id
        .get_or_insert_with(|| render_resource_context.create_sampler(&SamplerDescriptor::default()));

    let specialization = PipelineSpecialization {
        vertex_buffer_layout: VertexBufferLayout {
            name: Default::default(),
            stride: 36,
            step_mode: Default::default(),
            attributes: vec![
                VertexAttribute {
                    name: "Vertex_Position".into(),
                    offset: 0,
                    format: VertexFormat::Float2,
                    shader_location: 0,
                },
                VertexAttribute {
                    name: "Vertex_Uv".into(),
                    offset: 8,
                    format: VertexFormat::Float2,
                    shader_location: 1,
                },
                VertexAttribute {
                    name: "Vertex_Color".into(),
                    offset: 16,
                    format: VertexFormat::Float4,
                    shader_location: 2,
                },
                VertexAttribute {
                    name: "Vertex_Mode".into(),
                    offset: 32,
                    format: VertexFormat::Uint,
                    shader_location: 3,
                },
            ],
        },
        ..PipelineSpecialization::default()
    };

    let typed_handle = UI_PIPELINE_HANDLE.typed();
    let pipeline =
        if let Some(pipeline) = pipeline_compiler.get_specialized_pipeline(&typed_handle, &specialization) {
            pipeline
        } else {
            pipeline_compiler.compile_pipeline(
                &**render_resource_context,
                &mut pipelines,
                &mut shaders,
                &typed_handle,
                &specialization,
            )
        };

    let pipeline_descriptor = pipelines.get(&pipeline).unwrap();
    let bind_group_descriptor = pipeline_descriptor.get_layout().unwrap().get_bind_group(0).unwrap();

    draw.clear();
    draw.push(RenderCommand::SetPipeline { pipeline });
    let mut bind_group_set = false;

    for (mut ui_draw, stylesheet) in query.iter_mut() {
        let textures = if let Some(&mut Stylesheet { ref mut textures, .. }) = stylesheets.get_mut(stylesheet) {
            textures
        } else {
            continue;
        };

        let mut new_textures = HashMap::default();
        let mut updates = Vec::default();

        for update in ui_draw.updates.drain(..) {
            match update {
                Update::Texture { id, size, data, atlas } => {
                    new_textures.insert(id, (size, data, atlas));
                }
                Update::TextureSubresource { id, offset, size, data } => {
                    updates.push((id, offset, size, data));
                }
            }
        }

        for (id, (size, data, _atlas)) in new_textures {
            let size = Extent3d {
                width: size[0],
                height: size[1],
                depth: 1,
            };

            let padding = 256 - (size.width * 4) % 256;
            let data = if padding > 0 {
                data.chunks(size.width as usize * 4).fold(Vec::new(), |mut data, row| {
                    data.extend_from_slice(row);
                    data.extend(std::iter::repeat(0).take(padding as _));
                    data
                })
            } else {
                data
            };

            let texture_id = render_resource_context.create_texture(TextureDescriptor {
                size,
                ..TextureDescriptor::default()
            });

            if let Some(overwritten) = textures.insert(id, texture_id) {
                render_resource_context.remove_texture(overwritten);
            }

            if !data.is_empty() {
                let texture_data = render_resource_context.create_buffer_with_data(
                    BufferInfo {
                        size: data.len(),
                        buffer_usage: BufferUsage::COPY_SRC,
                        mapped_at_creation: false,
                    },
                    data.as_slice(),
                );

                state.command_queue.copy_buffer_to_texture(
                    texture_data,
                    0,
                    size.width * 4 + padding,
                    texture_id,
                    [0; 3],
                    0,
                    size,
                );
            }
        }

        for (id, offset, size, data) in updates {
            let size = Extent3d {
                width: size[0],
                height: size[1],
                depth: 1,
            };

            let padding = 256 - (size.width * 4) % 256;
            let data = if padding > 0 {
                data.chunks(size.width as usize * 4).fold(Vec::new(), |mut data, row| {
                    data.extend_from_slice(row);
                    data.extend(std::iter::repeat(0).take(padding as _));
                    data
                })
            } else {
                data
            };

            let texture_data = render_resource_context.create_buffer_with_data(
                BufferInfo {
                    size: data.len(),
                    buffer_usage: BufferUsage::COPY_SRC,
                    mapped_at_creation: false,
                },
                data.as_slice(),
            );

            let texture_id = textures.get(&id).cloned().unwrap();

            state.command_queue.copy_buffer_to_texture(
                texture_data,
                0,
                size.width * 4 + padding,
                texture_id,
                [offset[0], offset[1], 0],
                0,
                size,
            );
        }

        if ui_draw.vertices.is_some() {
            draw.push(RenderCommand::SetVertexBuffer {
                slot: 0,
                buffer: ui_draw.vertices.unwrap(),
                offset: 0
            });
            draw.push(RenderCommand::SetScissorRect {
                x: 0,
                y: 0,
                w: window.physical_width(),
                h: window.physical_height(),
            });

            for command in ui_draw.commands.iter() {
                match command {
                    pixel_widgets::draw::Command::Nop => (),
                    pixel_widgets::draw::Command::Clip { scissor } => {
                        let scale = window.scale_factor() as f32;
                        draw.push(RenderCommand::SetScissorRect {
                            x: (scissor.left * scale) as u32,
                            y: (scissor.top * scale) as u32,
                            w: (scissor.width() * scale) as u32,
                            h: (scissor.height() * scale) as u32,
                        })
                    }
                    &pixel_widgets::draw::Command::Colored { offset, count } => {
                        if !bind_group_set {
                            // just create a bind group for the first texture
                            let first_texture = textures.iter().next().unwrap();
                            render_resource_bindings.set("t_Color", RenderResourceBinding::Texture(*first_texture.1));
                            render_resource_bindings.set("s_Color", RenderResourceBinding::Sampler(sampler_id));
                            render_resource_bindings
                                .update_bind_groups(pipeline_descriptor, &**render_resource_context);
                            let bind_group = render_resource_bindings
                                .get_descriptor_bind_group(bind_group_descriptor.id)
                                .unwrap();
                            draw.push(RenderCommand::SetBindGroup {
                                index: bind_group_descriptor.index,
                                bind_group: bind_group.id,
                                dynamic_uniform_indices: None
                            });

                            bind_group_set = true;
                        }
                        draw.push(RenderCommand::Draw {
                            vertices: (offset as u32)..(offset + count) as u32,
                            instances: 0..1,
                        });
                    }
                    &pixel_widgets::draw::Command::Textured { texture, offset, count } => {
                        let texture = textures.get(&texture).cloned().unwrap();
                        render_resource_bindings.set("t_Color", RenderResourceBinding::Texture(texture));
                        render_resource_bindings.set("s_Color", RenderResourceBinding::Sampler(sampler_id));
                        render_resource_bindings.update_bind_groups(pipeline_descriptor, &**render_resource_context);
                        let bind_group = render_resource_bindings
                            .get_descriptor_bind_group(bind_group_descriptor.id)
                            .unwrap();
                        draw.push(RenderCommand::SetBindGroup {
                            index: bind_group_descriptor.index,
                            bind_group: bind_group.id,
                            dynamic_uniform_indices: None
                        });

                        bind_group_set = true;

                        draw.push(RenderCommand::Draw {
                            vertices: (offset as u32)..(offset + count) as u32,
                            instances: 0..1,
                        });
                    }
                }
            }
        }
    }

    *state.command_buffer.lock().unwrap() = draw;
}

/// Tracks the current pipeline state to ensure draw calls are valid.
#[derive(Debug, Default)]
struct DrawState {
    pipeline: Option<Handle<PipelineDescriptor>>,
    bind_groups: Vec<Option<BindGroupId>>,
    vertex_buffers: Vec<Option<BufferId>>,
    index_buffer: Option<BufferId>,
}

impl DrawState {
    pub fn set_bind_group(&mut self, index: u32, bind_group: BindGroupId) {
        self.bind_groups[index as usize] = Some(bind_group);
    }

    pub fn set_vertex_buffer(&mut self, index: u32, buffer: BufferId) {
        self.vertex_buffers[index as usize] = Some(buffer);
    }

    pub fn can_draw(&self) -> bool {
        self.bind_groups.iter().all(|b| b.is_some()) && self.vertex_buffers.iter().all(|v| v.is_some())
    }

    pub fn set_pipeline(&mut self, handle: &Handle<PipelineDescriptor>, descriptor: &PipelineDescriptor) {
        self.bind_groups.clear();
        self.vertex_buffers.clear();
        self.index_buffer = None;

        self.pipeline = Some(handle.clone_weak());
        let layout = descriptor.get_layout().unwrap();

        self.bind_groups.resize(layout.bind_groups.len(), None);
        self.vertex_buffers.resize(layout.vertex_buffer_descriptors.len(), None);
    }
}
