use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::Mutex;

use bevy::prelude::*;
use bevy::render::draw::RenderCommand;
use bevy::render::render_graph::base::MainPass;
use bevy::render::renderer::*;
use bevy::render::texture::{Extent3d, SamplerDescriptor, TextureDescriptor};
use pixel_widgets::draw::{DrawList, Update, Vertex};
use pixel_widgets::layout::Rectangle;
use pixel_widgets::loader::FsLoader;
use pixel_widgets::{Command, EventLoop, Model, Ui};
use zerocopy::AsBytes;

mod node;
mod pipeline;
mod plugin;
mod event;

pub mod prelude {
    pub use super::{UiComponent, UiComponents, UiPlugin};
    pub use pixel_widgets::prelude::*;
}

pub use pixel_widgets;

pub struct UiPlugin<M: Model + Send + Sync>(PhantomData<M>);

pub struct UiComponent<M: Model + Send + Sync> {
    pub ui: Ui<M, EventSender<M>, FsLoader>,
    receiver: Mutex<Receiver<Command<M::Message>>>,
    draw_commands: Vec<pixel_widgets::draw::Command>,
    vertex_buffer: Option<BufferId>,
    textures: HashMap<usize, TextureId>,
}

pub struct UiComponents<M: Model + Send + Sync> {
    pub component: UiComponent<M>,
    pub main_pass: MainPass,
    pub draw: Draw,
}

pub struct EventSender<M: Model + Send + Sync> {
    sender: SyncSender<Command<M::Message>>,
}

impl<M: Model + Send + Sync> EventLoop<Command<M::Message>> for EventSender<M> {
    type Error = std::sync::mpsc::SendError<Command<M::Message>>;

    fn send_event(&self, event: Command<M::Message>) -> Result<(), Self::Error> {
        self.sender.send(event)
    }
}

impl<M: Model + Send + Sync> Clone for EventSender<M> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<M: Model + Send + Sync> UiComponents<M> {
    pub fn new(model: M) -> Self {
        let loader = FsLoader::new(PathBuf::from(".")).unwrap();
        let (sender, receiver) = std::sync::mpsc::sync_channel(100);

        UiComponents {
            component: UiComponent {
                ui: Ui::new(model, EventSender { sender }, loader, Rectangle::from_wh(1280.0, 720.0)),
                draw_commands: Vec::new(),
                vertex_buffer: None,
                textures: HashMap::new(),
                receiver: Mutex::new(receiver),
            },
            main_pass: Default::default(),
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
        }
    }

    pub fn spawn(self, commands: &mut Commands) -> Entity {
        commands
            .spawn(())
            .with(self.component)
            .with(self.draw)
            .with(self.main_pass)
            .current_entity()
            .unwrap()
    }
}
