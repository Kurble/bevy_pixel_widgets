use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::render::pipeline::*;
use bevy::render::render_graph::{CommandQueue, Node, ResourceSlots, SystemNode};
use bevy::render::renderer::RenderContext;
use pixel_widgets::Model;

use super::*;
use crate::pipeline::UI_PIPELINE_HANDLE;

pub struct UiNode<M: Model + Send + Sync> {
    command_queue: CommandQueue,
    _marker: PhantomData<M>,
}

impl<M: Model + Send + Sync> Node for UiNode<M> {
    fn update(
        &mut self,
        _world: &World,
        _resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        self.command_queue.execute(render_context);
    }
}

impl<M: Model + Send + Sync> SystemNode for UiNode<M> {
    fn get_system(&self, commands: &mut Commands) -> Box<dyn System> {
        let system = render_ui::<M>.system();
        commands.insert_local_resource(
            system.id(),
            State {
                command_queue: self.command_queue.clone(),
                sampler_id: None,
            },
        );

        system
    }
}

impl<M: Model + Send + Sync> Default for UiNode<M> {
    fn default() -> Self {
        Self {
            command_queue: Default::default(),
            _marker: Default::default(),
        }
    }
}

#[derive(Default)]
struct State {
    command_queue: CommandQueue,
    sampler_id: Option<SamplerId>,
}

fn render_ui<M: Model + Send + Sync>(
    mut state: Local<State>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut pipeline_compiler: ResMut<PipelineCompiler>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut ui: Mut<UiComponent<M>>,
    mut draw: Mut<Draw>,
) {
    let &mut UiComponent {
        ref mut textures,
        ref mut draw_commands,
        ref mut vertex_buffer,
        ref mut ui,
        ..
    } = &mut *ui;

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

        let old_buffer = vertex_buffer.replace(render_resource_context.create_buffer_with_data(
            BufferInfo {
                size: vertices.len() * std::mem::size_of::<Vertex>(),
                buffer_usage: BufferUsage::VERTEX,
                mapped_at_creation: false,
            },
            vertices.as_bytes(),
        ));

        if let Some(old_buffer) = old_buffer {
            render_resource_context.remove_buffer(old_buffer);
        }

        *draw_commands = commands;
    }

    let sampler_id = *state
        .sampler_id
        .get_or_insert_with(|| render_resource_context.create_sampler(&SamplerDescriptor::default()));

    let pipeline = if let Some(pipeline) =
        pipeline_compiler.get_specialized_pipeline(UI_PIPELINE_HANDLE, &PipelineSpecialization::default())
    {
        pipeline
    } else {
        let mut descriptors = VertexBufferDescriptors::default();
        descriptors.set(VertexBufferDescriptor {
            name: "Vertex".into(),
            stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: InputStepMode::Vertex,
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
        });
        pipeline_compiler.compile_pipeline(
            &**render_resource_context,
            &mut pipelines,
            &mut shaders,
            UI_PIPELINE_HANDLE,
            &descriptors,
            &PipelineSpecialization::default(),
        )
    };

    draw.set_pipeline(pipeline);
    draw.set_vertex_buffer(0, vertex_buffer.unwrap(), 0);

    let bind_group_descriptor_id = pipelines.get(&pipeline).unwrap().get_layout().unwrap().bind_groups[0].id;
    let mut bind_groups = HashMap::new();

    let mut current_scissor = None;
    for command in draw_commands.iter() {
        match command {
            &pixel_widgets::draw::Command::Nop => (),
            &pixel_widgets::draw::Command::Clip { scissor } => {
                // a bit sad that we can't really use this atm... no scrolling!
                current_scissor.replace(scissor);
            }
            &pixel_widgets::draw::Command::Colored { offset, count } => {
                if bind_groups.is_empty() {
                    // just create a bind group for the first texture
                    let first_texture = textures.iter().next().unwrap();
                    let bind_group = bind_groups.entry(*first_texture.0).or_insert_with(|| {
                        let bind_group = BindGroup::build()
                            .add_texture(0, *first_texture.1)
                            .add_sampler(1, sampler_id)
                            .finish();
                        render_resource_context.create_bind_group(bind_group_descriptor_id, &bind_group);
                        bind_group
                    });
                    draw.set_bind_group(0, bind_group);
                }
                draw.render_command(RenderCommand::Draw {
                    vertices: (offset as u32)..(offset + count) as u32,
                    instances: 0..1,
                });
            }
            &pixel_widgets::draw::Command::Textured { texture, offset, count } => {
                let bind_group = bind_groups.entry(texture).or_insert_with(|| {
                    let bind_group = BindGroup::build()
                        .add_texture(0, textures.get(&texture).cloned().unwrap())
                        .add_sampler(1, sampler_id)
                        .finish();
                    render_resource_context.create_bind_group(bind_group_descriptor_id, &bind_group);
                    bind_group
                });
                draw.set_bind_group(0, bind_group);
                draw.render_command(RenderCommand::Draw {
                    vertices: (offset as u32)..(offset + count) as u32,
                    instances: 0..1,
                });
            }
        }
    }
}
