use std::future::Future;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::Mutex;

use bevy::asset::{AssetIoError, Handle};
use bevy::ecs::Bundle;
use bevy::render::renderer::*;
use bevy::render::texture::{Extent3d, SamplerDescriptor, TextureDescriptor};
use pixel_widgets::{Command, EventLoop, Model};
pub use pixel_widgets::*;
use pixel_widgets::draw::Update;
use pixel_widgets::layout::Rectangle;
use pixel_widgets::loader::Loader;

mod pixel_widgets_node;
mod pipeline;
mod plugin;
mod event;
mod style;

pub mod prelude {
    pub use pixel_widgets::{
        Command,
        layout::Rectangle,
        Model,
        stylesheet::Style,
        tracker::ManagedState,
        widget::IntoNode
    };

    pub use super::{Ui, UiBundle, UiDraw, UiPlugin};
    pub use super::style::Stylesheet;
}

pub struct UiPlugin<M: Model + Send + Sync>(PhantomData<M>);

pub struct Ui<M: Model + Send + Sync> {
    ui: pixel_widgets::Ui<M, EventSender<M>, DisabledLoader>,
    receiver: Mutex<Receiver<Command<M::Message>>>,
}

#[derive(Default)]
pub struct UiDraw {
    vertices: Option<BufferId>,
    updates: Vec<pixel_widgets::draw::Update>,
    commands: Vec<pixel_widgets::draw::Command>,
}

#[derive(Bundle)]
pub struct UiBundle<M: Model + Send + Sync> {
    pub ui: Ui<M>,
    pub draw: UiDraw,
    pub stylesheet: Handle<style::Stylesheet>,
}

pub struct EventSender<M: Model + Send + Sync> {
    sender: SyncSender<Command<M::Message>>,
}

pub struct DisabledLoader;

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
        let (sender, receiver) = std::sync::mpsc::sync_channel(100);
        Ui {
            ui: pixel_widgets::Ui::new(model, EventSender { sender }, DisabledLoader, Rectangle::from_wh(1280.0, 720.0)),
            receiver: Mutex::new(receiver),
        }
    }
}

impl<M: Model + Send + Sync> Deref for Ui<M> {
    type Target = pixel_widgets::Ui<M, EventSender<M>, DisabledLoader>;

    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}

impl<M: Model + Send + Sync> DerefMut for Ui<M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}

impl Loader for DisabledLoader {
    #[allow(clippy::type_complexity)]
    type Load = Pin<Box<dyn Future<Output = Result<Vec<u8>, Self::Error>> + Send>>;
    type Wait = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>>;
    type Error = AssetIoError;

    fn load(&self, _: impl AsRef<str>) -> Self::Load {
        unimplemented!("please load stylesheets using the bevy asset system");
    }

    fn wait(&self, _: impl AsRef<str>) -> Self::Wait {
        unimplemented!("please load stylesheets using the bevy asset system");
    }
}