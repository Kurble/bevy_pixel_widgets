use bevy::input::keyboard::{ElementState, KeyboardInput};
use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::input::prelude::*;
use bevy::prelude::*;
use bevy::window::WindowResized;
use pixel_widgets::event::{Event, Key, Modifiers};
use pixel_widgets::prelude::*;

use crate::UiComponent;

pub struct State {
    keyboard: EventReader<KeyboardInput>,
    mouse_button: EventReader<MouseButtonInput>,
    cursor_move: EventReader<CursorMoved>,
    mouse_wheel: EventReader<MouseWheel>,
    window_resize: EventReader<WindowResized>,
    modifiers: Modifiers,
}

impl Default for State {
    fn default() -> Self {
        Self {
            keyboard: Default::default(),
            mouse_button: Default::default(),
            cursor_move: Default::default(),
            mouse_wheel: Default::default(),
            window_resize: Default::default(),
            modifiers: Modifiers {
                ctrl: false,
                alt: false,
                shift: false,
                logo: false,
            }
        }
    }
}

pub fn update_ui<M: Model + Send + Sync>(
    mut state: Local<State>,
    keyboard_events: Res<Events<KeyboardInput>>,
    mouse_button_events: Res<Events<MouseButtonInput>>,
    cursor_moved_events: Res<Events<CursorMoved>>,
    mouse_wheel_events: Res<Events<MouseWheel>>,
    window_resize_events: Res<Events<WindowResized>>,
    mut ui: Query<&mut UiComponent<M>>) {
    let mut events = Vec::new();

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
                    if let Some(text) = translate_key_to_text(key, state.modifiers) {
                        events.push(Event::Text(text));
                    }
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

    for event in state.cursor_move.iter(&cursor_moved_events) {
        events.push(Event::Cursor(event.position.x(), event.position.y()));
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

    for mut ui in &mut ui.iter() {
        let &mut UiComponent {
            ref mut ui,
            ref mut receiver,
            ..
        } = &mut *ui;

        // process async events
        for cmd in receiver.get_mut().unwrap().try_iter() {
            ui.command(cmd);
        }

        // process input events
        for event in events.iter() {
            ui.event(event.clone());
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

fn translate_key_to_text(key_code: Key, modifiers: Modifiers) -> Option<char> {
    match (key_code, modifiers.shift) {
        (Key::A, false) => Some('a'),
        (Key::B, false) => Some('b'),
        (Key::C, false) => Some('c'),
        (Key::D, false) => Some('d'),
        (Key::E, false) => Some('e'),
        (Key::F, false) => Some('f'),
        (Key::G, false) => Some('g'),
        (Key::H, false) => Some('h'),
        (Key::I, false) => Some('i'),
        (Key::J, false) => Some('j'),
        (Key::K, false) => Some('k'),
        (Key::L, false) => Some('l'),
        (Key::M, false) => Some('m'),
        (Key::N, false) => Some('n'),
        (Key::O, false) => Some('o'),
        (Key::P, false) => Some('p'),
        (Key::Q, false) => Some('q'),
        (Key::R, false) => Some('r'),
        (Key::S, false) => Some('s'),
        (Key::T, false) => Some('t'),
        (Key::U, false) => Some('u'),
        (Key::V, false) => Some('v'),
        (Key::W, false) => Some('w'),
        (Key::X, false) => Some('x'),
        (Key::Y, false) => Some('y'),
        (Key::Z, false) => Some('z'),
        (Key::A, true) => Some('A'),
        (Key::B, true) => Some('B'),
        (Key::C, true) => Some('C'),
        (Key::D, true) => Some('D'),
        (Key::E, true) => Some('E'),
        (Key::F, true) => Some('F'),
        (Key::G, true) => Some('G'),
        (Key::H, true) => Some('H'),
        (Key::I, true) => Some('I'),
        (Key::J, true) => Some('J'),
        (Key::K, true) => Some('K'),
        (Key::L, true) => Some('L'),
        (Key::M, true) => Some('M'),
        (Key::N, true) => Some('N'),
        (Key::O, true) => Some('O'),
        (Key::P, true) => Some('P'),
        (Key::Q, true) => Some('Q'),
        (Key::R, true) => Some('R'),
        (Key::S, true) => Some('S'),
        (Key::T, true) => Some('T'),
        (Key::U, true) => Some('U'),
        (Key::V, true) => Some('V'),
        (Key::W, true) => Some('W'),
        (Key::X, true) => Some('X'),
        (Key::Y, true) => Some('Y'),
        (Key::Z, true) => Some('Z'),
        (Key::Key0, false) => Some('0'),
        (Key::Key1, false) => Some('1'),
        (Key::Key2, false) => Some('2'),
        (Key::Key3, false) => Some('3'),
        (Key::Key4, false) => Some('4'),
        (Key::Key5, false) => Some('5'),
        (Key::Key6, false) => Some('6'),
        (Key::Key7, false) => Some('7'),
        (Key::Key8, false) => Some('8'),
        (Key::Key9, false) => Some('9'),
        (Key::Key0, true) => Some('!'),
        (Key::Key1, true) => Some('@'),
        (Key::Key2, true) => Some('#'),
        (Key::Key3, true) => Some('$'),
        (Key::Key4, true) => Some('%'),
        (Key::Key5, true) => Some('^'),
        (Key::Key6, true) => Some('&'),
        (Key::Key7, true) => Some('*'),
        (Key::Key8, true) => Some('('),
        (Key::Key9, true) => Some(')'),
        (Key::Space, _) => Some(' '),
        _ => None,
    }
}

fn translate_mouse_button(button: MouseButton) -> Option<Key> {
    Some(match button {
        MouseButton::Left => Key::LeftMouseButton,
        MouseButton::Right => Key::RightMouseButton,
        MouseButton::Middle => Key::MiddleMouseButton,
        _ => None?,
    })
}