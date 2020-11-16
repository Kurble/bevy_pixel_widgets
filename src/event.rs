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
    keyboard: EventReader<KeyboardInput>,
    characters: EventReader<ReceivedCharacter>,
    mouse_button: EventReader<MouseButtonInput>,
    cursor_move: EventReader<CursorMoved>,
    mouse_wheel: EventReader<MouseWheel>,
    window_resize: EventReader<WindowResized>,
    modifiers: Modifiers,
    current_window_size: Option<(f32, f32)>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            keyboard: Default::default(),
            characters: Default::default(),
            mouse_button: Default::default(),
            cursor_move: Default::default(),
            mouse_wheel: Default::default(),
            window_resize: Default::default(),
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

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_ui<M: Model + Send + Sync>(
    mut state: Local<State>,
    windows: Res<Windows>,
    keyboard_events: Res<Events<KeyboardInput>>,
    character_events: Res<Events<ReceivedCharacter>>,
    mouse_button_events: Res<Events<MouseButtonInput>>,
    cursor_moved_events: Res<Events<CursorMoved>>,
    mouse_wheel_events: Res<Events<MouseWheel>>,
    window_resize_events: Res<Events<WindowResized>>,
    stylesheets: Res<Assets<Stylesheet>>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut ui: Query<(&mut Ui<M>, &mut UiDraw, Option<&Handle<Stylesheet>>)>,
) {
    let mut events = Vec::new();
    let window = windows.get_primary().unwrap();
    let resize =
        Some((window.width() as f32, window.height() as f32)).filter(|&new| state.current_window_size != Some(new));
    state.current_window_size = Some((window.width() as f32, window.height() as f32));

    for event in state.window_resize.iter(&window_resize_events) {
        events.push(Event::Resize(event.width as f32, event.height as f32));
    }

    for event in state.keyboard.iter(&keyboard_events) {
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

    for event in state.characters.iter(&character_events) {
        events.push(Event::Text(event.char));
    }

    for event in state.cursor_move.iter(&cursor_moved_events) {
        events.push(Event::Cursor(event.position.x(), window.height() as f32 - event.position.y()));
    }

    for event in state.mouse_wheel.iter(&mouse_wheel_events) {
        events.push(Event::Scroll(event.x, event.y))
    }

    for event in state.mouse_button.iter(&mouse_button_events) {
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

    for (mut ui, mut draw, stylesheet) in ui.iter_mut() {
        let &mut Ui {
            ref mut ui,
            ref mut receiver,
        } = &mut *ui;

        if let Some((width, height)) = resize {
            ui.resize(Rectangle::from_wh(width, height));
        }

        if let Some(stylesheet) = stylesheet.and_then(|s| stylesheets.get(s)) {
            ui.replace_stylesheet(stylesheet.style.clone());
        }

        // process async events
        for cmd in receiver.get_mut().unwrap().try_iter() {
            ui.command(cmd);
        }

        // process input events
        for &event in events.iter() {
            ui.event(event);
        }

        // update ui drawing
        if ui.needs_redraw() {
            let DrawList {
                updates,
                commands,
                vertices,
            } = ui.draw();

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