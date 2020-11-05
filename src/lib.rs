use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::Mutex;
use std::ops::{Deref, DerefMut};

use bevy::render::renderer::*;
use bevy::render::texture::{Extent3d, SamplerDescriptor, TextureDescriptor};
use pixel_widgets::{Command, EventLoop, Model};
pub use pixel_widgets::*;
use pixel_widgets::draw::{DrawList, Update, Vertex};
use pixel_widgets::layout::Rectangle;
use pixel_widgets::loader::FsLoader;
use zerocopy::AsBytes;

mod pixel_widgets_node;
mod pipeline;
mod plugin;
mod event;

pub mod prelude {
    pub use pixel_widgets::{
        Command,
        layout::Rectangle,
        Model,
        stylesheet::Style,
        tracker::ManagedState,
        widget::IntoNode
    };

    pub use super::{Ui, UiPlugin};
}

pub struct UiPlugin<M: Model + Send + Sync>(PhantomData<M>);

pub struct Ui<M: Model + Send + Sync> {
    ui: pixel_widgets::Ui<M, EventSender<M>, FsLoader>,
    receiver: Mutex<Receiver<Command<M::Message>>>,
    draw_commands: Vec<pixel_widgets::draw::Command>,
    vertex_buffer: Option<BufferId>,
    textures: HashMap<usize, TextureId>,
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

impl<M: Model + Send + Sync> Ui<M> {
    pub fn new(model: M) -> Self {
        let loader = FsLoader::new(PathBuf::from(".")).unwrap();
        let (sender, receiver) = std::sync::mpsc::sync_channel(100);

        Ui {
            ui: pixel_widgets::Ui::new(model, EventSender { sender }, loader, Rectangle::from_wh(1280.0, 720.0)),
            draw_commands: Vec::default(),
            vertex_buffer: None,
            textures: HashMap::new(),
            receiver: Mutex::new(receiver),
        }
    }
}

impl<M: Model + Send + Sync> Deref for Ui<M> {
    type Target = pixel_widgets::Ui<M, EventSender<M>, FsLoader>;

    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}

impl<M: Model + Send + Sync> DerefMut for Ui<M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}
