use bevy::ecs::system::{
    ResState, SystemId, SystemParam, SystemParamFetch, SystemParamFunction, SystemParamState, SystemState,
};
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::input::prelude::*;
use bevy::input::ElementState;
use bevy::prelude::*;
use bevy::render::renderer::{BufferInfo, BufferUsage, RenderResourceContext};
use bevy::window::WindowResized;
use pixel_widgets::draw::{DrawList, Vertex};
use pixel_widgets::event::{Event, Key, Modifiers};
use pixel_widgets::prelude::*;
use zerocopy::AsBytes;

use crate::style::Stylesheet;
use crate::{Ui, UiDraw};
use bevy::ecs::archetype::Archetype;
use std::marker::PhantomData;

pub struct State {
    modifiers: Modifiers,
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
        }
    }
}

impl<'a, M: Model + Send + Sync + UpdateModel<'a>> Ui<M> {
    pub fn update_commands(&mut self, resources: &mut <M as UpdateModel<'a>>::State) {
        for cmd in self.receiver.get_mut().unwrap().try_iter() {
            self.ui.command(cmd, resources);
        }
    }
}

pub struct Generic<'a, T> {
    phantom: PhantomData<&'a ()>,
    param: T,
}

pub struct GenericState<T> {
    state: T,
}

impl<'a, T: SystemParam> SystemParam for Generic<'a, T> {
    type Fetch = GenericState<T::Fetch>;
}

unsafe impl<T: SystemParamState> SystemParamState for GenericState<T> {
    type Config = T::Config;

    fn init(world: &mut World, system_state: &mut SystemState, config: Self::Config) -> Self {
        Self {
            state: T::init(world, system_state, config),
        }
    }

    fn new_archetype(&mut self, archetype: &Archetype, system_state: &mut SystemState) {
        self.state.new_archetype(archetype, system_state);
    }

    fn apply(&mut self, world: &mut World) {
        self.state.apply(world);
    }

    fn default_config() -> Self::Config {
        T::default_config()
    }
}

impl<'a, T: SystemParamFetch<'a>> SystemParamFetch<'a> for GenericState<T> {
    type Item = Generic<'a, T::Item>;

    unsafe fn get_param(
        state: &'a mut Self,
        system_state: &'a SystemState,
        world: &'a World,
        change_tick: u32,
    ) -> Self::Item {
        Generic {
            phantom: PhantomData,
            param: T::get_param(&mut state.state, system_state, world, change_tick),
        }
    }
}

#[derive(SystemParam)]
pub struct UpdateUiSystemParams<'a, M: Model + Send + Sync> {
    pub state: Local<'a, State>,
    pub windows: Res<'a, Windows>,
    pub keyboard_events: EventReader<'a, KeyboardInput>,
    pub character_events: EventReader<'a, ReceivedCharacter>,
    pub mouse_button_events: EventReader<'a, MouseButtonInput>,
    pub cursor_moved_events: EventReader<'a, CursorMoved>,
    pub mouse_wheel_events: EventReader<'a, MouseWheel>,
    pub window_resize_events: EventReader<'a, WindowResized>,
    pub stylesheets: Res<'a, Assets<Stylesheet>>,
    pub render_resource_context: Res<'a, Box<dyn RenderResourceContext>>,
    pub query: Query<
        'a,
        (
            &'static mut Ui<M>,
            &'static mut UiDraw,
            Option<&'static Handle<Stylesheet>>,
        ),
    >,
}

pub struct UpdateUi<M, Param>(PhantomData<(M, Param)>);

pub fn update_ui_system<
    M: 'static + Model + Send + Sync + for<'a> UpdateModel<'a, State = <Param::Fetch as SystemParamFetch<'a>>::Item>,
    Param: 'static + SystemParam + Send + Sync,
>() -> impl System<In = (), Out = ()> {
    use bevy::ecs::system::IntoSystem;
    UpdateUi::<M, Param>(PhantomData).system()
}

impl<M, Param> SystemParamFunction<(), (), (UpdateUiSystemParams<'static, M>, Param), ()> for UpdateUi<M, Param>
where
    M: 'static + Model + Send + Sync + for<'a> UpdateModel<'a, State = <Param::Fetch as SystemParamFetch<'a>>::Item>,
    Param: 'static + SystemParam + Send + Sync,
    Self: 'static + Send + Sync,
{
    fn run(
        &mut self,
        _: (),
        state: &mut <(UpdateUiSystemParams<'_, M>, Param) as SystemParam>::Fetch,
        system_state: &SystemState,
        world: &World,
        change_tick: u32,
    ) -> () {
        let (mut params, mut custom): (UpdateUiSystemParams<M>, <Param::Fetch as SystemParamFetch<'_>>::Item) = unsafe {
            <<(UpdateUiSystemParams<'_, M>, Param) as SystemParam>::Fetch as SystemParamFetch<'_>>::get_param(
                state,
                system_state,
                world,
                change_tick,
            )
        };

        let mut events = Vec::new();
        let window = params.windows.get_primary().unwrap();

        for event in params.window_resize_events.iter() {
            events.push(Event::Resize(event.width as f32, event.height as f32));
        }

        for event in params.keyboard_events.iter() {
            match event.key_code {
                Some(KeyCode::LControl) | Some(KeyCode::RControl) => {
                    params.state.modifiers.ctrl = event.state == ElementState::Pressed;
                    events.push(Event::Modifiers(params.state.modifiers));
                }
                Some(KeyCode::LAlt) | Some(KeyCode::RAlt) => {
                    params.state.modifiers.alt = event.state == ElementState::Pressed;
                    events.push(Event::Modifiers(params.state.modifiers));
                }
                Some(KeyCode::LShift) | Some(KeyCode::RShift) => {
                    params.state.modifiers.shift = event.state == ElementState::Pressed;
                    events.push(Event::Modifiers(params.state.modifiers));
                }
                Some(KeyCode::LWin) | Some(KeyCode::RWin) => {
                    params.state.modifiers.shift = event.state == ElementState::Pressed;
                    events.push(Event::Modifiers(params.state.modifiers));
                }
                _ => (),
            }

            match event {
                KeyboardInput {
                    key_code,
                    state: ElementState::Pressed,
                    ..
                } => {
                    if let Some(key) = key_code.and_then(translate_key_code) {
                        events.push(Event::Press(key));
                    }
                }
                KeyboardInput {
                    key_code,
                    state: ElementState::Released,
                    ..
                } => {
                    if let Some(key) = key_code.and_then(translate_key_code) {
                        events.push(Event::Release(key));
                    }
                }
            }
        }

        for event in params.character_events.iter() {
            events.push(Event::Text(event.char));
        }

        for event in params.cursor_moved_events.iter() {
            events.push(Event::Cursor(
                event.position.x,
                window.height() as f32 - event.position.y,
            ));
        }

        for event in params.mouse_wheel_events.iter() {
            events.push(Event::Scroll(event.x, event.y))
        }

        for event in params.mouse_button_events.iter() {
            match event {
                MouseButtonInput {
                    button,
                    state: ElementState::Pressed,
                } => {
                    if let Some(key) = translate_mouse_button(*button) {
                        events.push(Event::Press(key));
                    }
                }
                MouseButtonInput {
                    button,
                    state: ElementState::Released,
                } => {
                    if let Some(key) = translate_mouse_button(*button) {
                        events.push(Event::Release(key));
                    }
                }
            }
        }

        for (mut wrapper, mut draw, stylesheet) in params.query.iter_mut() {
            if Some((window.width() as f32, window.height() as f32)) != wrapper.window {
                wrapper.window = Some((window.width() as f32, window.height() as f32));
                wrapper
                    .ui
                    .resize(Rectangle::from_wh(window.width() as f32, window.height() as f32));
            }

            if let Some(stylesheet) = stylesheet {
                if let Some(stylesheet) = params.stylesheets.get(stylesheet) {
                    wrapper.ui.replace_stylesheet(stylesheet.style.clone());
                }
            }

            // process async events
            wrapper.update_commands(&mut custom);

            // process input events
            for &event in events.iter() {
                wrapper.ui.event(event, &mut custom);
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
                    let old_buffer = draw
                        .vertices
                        .replace(params.render_resource_context.create_buffer_with_data(
                            BufferInfo {
                                size: vertices.len() * std::mem::size_of::<Vertex>(),
                                buffer_usage: BufferUsage::VERTEX,
                                mapped_at_creation: false,
                            },
                            vertices.as_bytes(),
                        ));

                    if let Some(b) = old_buffer {
                        params.render_resource_context.remove_buffer(b)
                    }
                } else if let Some(b) = draw.vertices.take() {
                    params.render_resource_context.remove_buffer(b)
                }
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
