use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy::render::draw::RenderCommand;
use bevy::render::pass::*;
use bevy::render::pipeline::*;
use bevy::render::render_graph::{CommandQueue, Node, ResourceSlotInfo, ResourceSlots, SystemNode};
use bevy::render::renderer::RenderContext;
use pixel_widgets::Model;

use crate::pipeline::UI_PIPELINE_HANDLE;

use super::*;

pub struct UiNode<M: Model + Send + Sync> {
    command_queue: CommandQueue,
    command_buffer: Arc<Mutex<Vec<RenderCommand>>>,
    descriptor: PassDescriptor,
    inputs: Vec<ResourceSlotInfo>,
    color_attachment_input_indices: Vec<Option<usize>>,
    color_resolve_target_indices: Vec<Option<usize>>,
    depth_stencil_attachment_input_index: Option<usize>,
    _marker: PhantomData<M>,
}

impl<M: Model + Send + Sync> Node for UiNode<M> {
    fn input(&self) -> &[ResourceSlotInfo] {
        &self.inputs
    }

    fn update(
        &mut self,
        _world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        self.command_queue.execute(render_context);

        let render_resource_bindings = resources.get::<RenderResourceBindings>().unwrap();
        let pipelines = resources.get::<Assets<PipelineDescriptor>>().unwrap();

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
                    RenderCommand::SetVertexBuffer { slot, buffer, offset } => {
                        pass.set_vertex_buffer(slot, buffer, offset);
                        draw_state.set_vertex_buffer(slot, buffer);
                    }
                    RenderCommand::SetIndexBuffer { buffer, offset } => {
                        pass.set_index_buffer(buffer, offset);
                        draw_state.set_index_buffer(buffer);
                    },
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
                            dynamic_uniform_indices
                                .as_ref()
                                .map(|indices| indices.deref()),
                        );
                        draw_state.set_bind_group(index, bind_group);
                    }
                    RenderCommand::DrawIndexed {
                        indices,
                        base_vertex,
                        instances,
                    } => {
                        if draw_state.can_draw_indexed() {
                            pass.draw_indexed(indices, base_vertex, instances);
                        } else {
                            println!("Could not draw indexed because the pipeline layout wasn't fully set for pipeline: {:?}", draw_state.pipeline);
                        }
                    },
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

impl<M: Model + Send + Sync> SystemNode for UiNode<M> {
    fn get_system(&self, commands: &mut Commands) -> Box<dyn System> {
        let system = render_ui::<M>.system();
        commands.insert_local_resource(
            system.id(),
            State {
                command_queue: self.command_queue.clone(),
                command_buffer: self.command_buffer.clone(),
                current_window_size: None,
                sampler_id: None,
            },
        );

        system
    }
}

impl<M: Model + Send + Sync> UiNode<M> {
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
            _marker: Default::default(),
        }
    }
}

#[derive(Default)]
struct State {
    command_queue: CommandQueue,
    command_buffer: Arc<Mutex<Vec<RenderCommand>>>,
    current_window_size: Option<(f32, f32)>,
    sampler_id: Option<SamplerId>,
}

fn render_ui<M: Model + Send + Sync>(
    mut state: Local<State>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut pipeline_compiler: ResMut<PipelineCompiler>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    windows: Res<Windows>,
    mut query: Query<&mut Ui<M>>,
) {
    let window = windows.get_primary().unwrap();
    let new_window_size =
        Some((window.width() as f32, window.height() as f32)).filter(|&new| state.current_window_size != Some(new));

    let mut draw = {
        let mut command_buffer = state.command_buffer.lock().unwrap();
        command_buffer.clear();
        Draw {
            is_visible: false,
            is_transparent: false,
            render_commands: std::mem::replace(&mut command_buffer, Vec::new()),
        }
    };

    let sampler_id = *state
        .sampler_id
        .get_or_insert_with(|| render_resource_context.create_sampler(&SamplerDescriptor::default()));

    let specialization = PipelineSpecialization {
        vertex_buffer_descriptor: VertexBufferDescriptor {
            name: Default::default(),
            stride: 36,
            step_mode: Default::default(),
            attributes: vec![
                VertexAttributeDescriptor {
                    name: "Vertex_Position".into(),
                    offset: 0,
                    format: VertexFormat::Float2,
                    shader_location: 0,
                },
                VertexAttributeDescriptor {
                    name: "Vertex_Uv".into(),
                    offset: 8,
                    format: VertexFormat::Float2,
                    shader_location: 1,
                },
                VertexAttributeDescriptor {
                    name: "Vertex_Color".into(),
                    offset: 16,
                    format: VertexFormat::Float4,
                    shader_location: 2,
                },
                VertexAttributeDescriptor {
                    name: "Vertex_Mode".into(),
                    offset: 32,
                    format: VertexFormat::Uint,
                    shader_location: 3,
                },
            ],
        },
        ..PipelineSpecialization::default()
    };

    let pipeline =
        if let Some(pipeline) = pipeline_compiler.get_specialized_pipeline(&UI_PIPELINE_HANDLE, &specialization) {
            pipeline
        } else {
            pipeline_compiler.compile_pipeline(
                &**render_resource_context,
                &mut pipelines,
                &mut shaders,
                &UI_PIPELINE_HANDLE,
                &specialization,
            )
        };

    let pipeline_descriptor = pipelines.get(&pipeline).unwrap();
    let bind_group_descriptor = pipeline_descriptor.get_layout().unwrap().get_bind_group(0).unwrap();

    draw.clear_render_commands();
    draw.set_pipeline(&pipeline);
    let mut bind_group_set = false;

    for mut ui in query.iter_mut() {
        let &mut Ui {
            ref mut textures,
            ref mut draw_commands,
            ref mut vertex_buffer,
            ref mut ui,
            ..
        } = &mut *ui;

        new_window_size.map(|(w, h)| ui.resize(Rectangle::from_wh(w, h)));

        if ui.needs_redraw() {
            // modify the mesh
            let DrawList {
                updates,
                commands,
                vertices,
            } = ui.draw();

            for update in updates {
                match update {
                    Update::TextureSubresource { id, offset, size, data } => {
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
                    Update::Texture { id, size, data, .. } => {
                        let size = Extent3d {
                            width: size[0],
                            height: size[1],
                            depth: 1,
                        };

                        let texture_id = render_resource_context.create_texture(TextureDescriptor {
                            size,
                            ..TextureDescriptor::default()
                        });

                        textures.insert(id, texture_id);

                        if data.len() > 0 {
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
                                size.width * 4,
                                texture_id,
                                [0; 3],
                                0,
                                size,
                            );
                        }
                    }
                }
            }

            if vertices.len() > 0 {
                let old_buffer = vertex_buffer.replace(render_resource_context.create_buffer_with_data(
                    BufferInfo {
                        size: vertices.len() * std::mem::size_of::<Vertex>(),
                        buffer_usage: BufferUsage::VERTEX,
                        mapped_at_creation: false,
                    },
                    vertices.as_bytes(),
                ));

                old_buffer.map(|b| render_resource_context.remove_buffer(b));
            } else {
                vertex_buffer.take().map(|b| render_resource_context.remove_buffer(b));
            }

            *draw_commands = commands;
        }

        if vertex_buffer.is_some() {
            draw.set_vertex_buffer(0, vertex_buffer.unwrap(), 0);

            for command in draw_commands.iter() {
                match command {
                    &pixel_widgets::draw::Command::Nop => (),
                    &pixel_widgets::draw::Command::Clip { .. } => {
                        // a bit sad that we can't really use this atm... no scrolling!
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
                            draw.set_bind_group(bind_group_descriptor.index, bind_group);

                            bind_group_set = true;
                        }
                        draw.render_command(RenderCommand::Draw {
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
                        draw.set_bind_group(bind_group_descriptor.index, bind_group);

                        bind_group_set = true;

                        draw.render_command(RenderCommand::Draw {
                            vertices: (offset as u32)..(offset + count) as u32,
                            instances: 0..1,
                        });
                    }
                }
            }
        }
    }

    *state.command_buffer.lock().unwrap() = draw.render_commands;
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

    pub fn set_index_buffer(&mut self, buffer: BufferId) {
        self.index_buffer = Some(buffer);
    }

    pub fn can_draw(&self) -> bool {
        self.bind_groups.iter().all(|b| b.is_some()) && self.vertex_buffers.iter().all(|v| v.is_some())
    }

    pub fn can_draw_indexed(&self) -> bool {
        self.can_draw() && self.index_buffer.is_some()
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
