use bevy::input::keyboard::{KeyboardInput};
use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::input::ElementState;
use bevy::input::prelude::*;
use bevy::prelude::*;
use bevy::window::WindowResized;
use pixel_widgets::event::{Event, Key, Modifiers};
use pixel_widgets::prelude::*;

use crate::{Ui, UiDraw};
use crate::style::Stylesheet;
use pixel_widgets::draw::{DrawList, Vertex};
use bevy::render::renderer::{RenderResourceContext, BufferUsage, BufferInfo};
use zerocopy::AsBytes;

pub struct State {
    modifiers: Modifiers,
    current_window_size: Option<(f32, f32)>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            modifiers: Modifiers {
                ctrl: false,
                alt: false,
                shift: false,
                logo: false,
            },
            current_window_size: Default::default(),
        }
    }
}

impl<M: Model + Send + Sync> Ui<M> {
    pub fn update_commands(&mut self) {
        for cmd in self.receiver.get_mut().unwrap().try_iter() {
            self.ui.command(cmd);
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_ui<M: Model + Send + Sync>(
    mut state: Local<State>,
    windows: Res<Windows>,
    mut keyboard_events: EventReader<KeyboardInput>,
    mut character_events: EventReader<ReceivedCharacter>,
    mut mouse_button_events: EventReader<MouseButtonInput>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut window_resize_events: EventReader<WindowResized>,
    stylesheets: Res<Assets<Stylesheet>>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut ui: Query<(&mut Ui<M>, &mut UiDraw, Option<&Handle<Stylesheet>>)>,
) {
    let mut events = Vec::new();
    let window = windows.get_primary().unwrap();
    let resize =
        Some((window.width() as f32, window.height() as f32)).filter(|&new| state.current_window_size != Some(new));
    state.current_window_size = Some((window.width() as f32, window.height() as f32));

    for event in window_resize_events.iter() {
        events.push(Event::Resize(event.width as f32, event.height as f32));
    }

    for event in keyboard_events.iter() {
        match event.key_code {
            Some(KeyCode::LControl) | Some(KeyCode::RControl) => {
                state.modifiers.ctrl = event.state == ElementState::Pressed;
                events.push(Event::Modifiers(state.modifiers));
            }
            Some(KeyCode::LAlt) | Some(KeyCode::RAlt) => {
                state.modifiers.alt = event.state == ElementState::Pressed;
                events.push(Event::Modifiers(state.modifiers));
            }
            Some(KeyCode::LShift) | Some(KeyCode::RShift) => {
                state.modifiers.shift = event.state == ElementState::Pressed;
                events.push(Event::Modifiers(state.modifiers));
            }
            Some(KeyCode::LWin) | Some(KeyCode::RWin) => {
                state.modifiers.shift = event.state == ElementState::Pressed;
                events.push(Event::Modifiers(state.modifiers));
            }
            _ => (),
        }

        match event {
            KeyboardInput { key_code, state: ElementState::Pressed, .. } => {
                if let Some(key) = key_code.and_then(translate_key_code) {
                    events.push(Event::Press(key));
                }
            }
            KeyboardInput { key_code, state: ElementState::Released, .. } => {
                if let Some(key) = key_code.and_then(translate_key_code) {
                    events.push(Event::Release(key));
                }
            }
        }
    }

    for event in character_events.iter() {
        events.push(Event::Text(event.char));
    }

    for event in cursor_moved_events.iter() {
        events.push(Event::Cursor(event.position.x, window.height() as f32 - event.position.y));
    }

    for event in mouse_wheel_events.iter() {
        events.push(Event::Scroll(event.x, event.y))
    }

    for event in mouse_button_events.iter() {
        match event {
            MouseButtonInput { button, state: ElementState::Pressed } => {
                if let Some(key) = translate_mouse_button(*button) {
                    events.push(Event::Press(key));
                }
            }
            MouseButtonInput { button, state: ElementState::Released } => {
                if let Some(key) = translate_mouse_button(*button) {
                    events.push(Event::Release(key));
                }
            }
        }
    }

    for (mut wrapper, mut draw, stylesheet) in ui.iter_mut() {
        if let Some((width, height)) = resize {
            wrapper.ui.resize(Rectangle::from_wh(width, height));
        }

        if let Some(stylesheet) = stylesheet.and_then(|s| stylesheets.get(s)) {
            wrapper.ui.replace_stylesheet(stylesheet.style.clone());
        }

        // process async events
        wrapper.update_commands();

        // process input events
        for &event in events.iter() {
            wrapper.ui.event(event);
        }

        // update ui drawing
        if wrapper.ui.needs_redraw() {
            let DrawList {
                updates,
                commands,
                vertices,
            } = wrapper.ui.draw();

            draw.updates.extend(updates.into_iter());
            draw.commands = commands;
            if !vertices.is_empty() {
                let old_buffer = draw.vertices.replace(render_resource_context.create_buffer_with_data(
                    BufferInfo {
                        size: vertices.len() * std::mem::size_of::<Vertex>(),
                        buffer_usage: BufferUsage::VERTEX,
                        mapped_at_creation: false,
                    },
                    vertices.as_bytes(),
                ));

                if let Some(b) = old_buffer {
                    render_resource_context.remove_buffer(b)
                }
            } else if let Some(b) = draw.vertices.take() {
                render_resource_context.remove_buffer(b)
            }
        }
    }
}

fn translate_key_code(key_code: KeyCode) -> Option<Key> {
    Some(match key_code {
        KeyCode::Key1 => Key::Key1,
        KeyCode::Key2 => Key::Key2,
        KeyCode::Key3 => Key::Key3,
        KeyCode::Key4 => Key::Key4,
        KeyCode::Key5 => Key::Key5,
        KeyCode::Key6 => Key::Key6,
        KeyCode::Key7 => Key::Key7,
        KeyCode::Key8 => Key::Key8,
        KeyCode::Key9 => Key::Key9,
        KeyCode::Key0 => Key::Key0,
        KeyCode::A => Key::A,
        KeyCode::B => Key::B,
        KeyCode::C => Key::C,
        KeyCode::D => Key::D,
        KeyCode::E => Key::E,
        KeyCode::F => Key::F,
        KeyCode::G => Key::G,
        KeyCode::H => Key::H,
        KeyCode::I => Key::I,
        KeyCode::J => Key::J,
        KeyCode::K => Key::K,
        KeyCode::L => Key::L,
        KeyCode::M => Key::M,
        KeyCode::N => Key::N,
        KeyCode::O => Key::O,
        KeyCode::P => Key::P,
        KeyCode::Q => Key::Q,
        KeyCode::R => Key::R,
        KeyCode::S => Key::S,
        KeyCode::T => Key::T,
        KeyCode::U => Key::U,
        KeyCode::V => Key::V,
        KeyCode::W => Key::W,
        KeyCode::X => Key::X,
        KeyCode::Y => Key::Y,
        KeyCode::Z => Key::Z,
        KeyCode::Escape => Key::Escape,
        KeyCode::Tab => Key::Tab,
        KeyCode::LShift => Key::Shift,
        KeyCode::LControl => Key::Ctrl,
        KeyCode::LAlt => Key::Alt,
        KeyCode::Space => Key::Space,
        KeyCode::Return => Key::Enter,
        KeyCode::Back => Key::Backspace,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        _ => None?,
    })
}

fn translate_mouse_button(button: MouseButton) -> Option<Key> {
    Some(match button {
        MouseButton::Left => Key::LeftMouseButton,
        MouseButton::Right => Key::RightMouseButton,
        MouseButton::Middle => Key::MiddleMouseButton,
        _ => None?,
    })
}