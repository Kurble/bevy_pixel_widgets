use bevy::prelude::*;
use bevy_pixel_widgets::prelude::*;
use bevy_pixel_widgets::{widget, UpdateModel};

struct Counter {
    pub value: i32,
    pub state: ManagedState<String>,
}

#[derive(Clone)]
enum Message {
    UpPressed,
    DownPressed,
}

impl Model for Counter {
    type Message = Message;

    fn view(&mut self) -> widget::Node<Message> {
        let mut state = self.state.tracker();
        widget::Scroll::new(
            state.get("scroll"),
            widget::Column::new()
                .push(
                    widget::Button::new(state.get("up"), widget::Text::new("Up"))
                        .on_clicked(Message::UpPressed),
                )
                .push(widget::Text::new(format!("Count: {}", self.value)))
                .push(
                    widget::Button::new(state.get("down"), widget::Text::new("Down"))
                        .on_clicked(Message::DownPressed),
                ),
        )
        .into_node()
    }
}

impl<'a> UpdateModel<'a> for Counter {
    type State = ();

    fn update(&mut self, message: Self::Message, _: &mut Self::State) -> Vec<Command<Message>> {
        match message {
            Message::UpPressed => {
                self.value += 1;
                Vec::new()
            }
            Message::DownPressed => {
                self.value -= 1;
                Vec::new()
            }
        }
    }
}

fn update_counter(params: UpdateUiSystemParams<Counter>, state: ()) {
    params.update(state);
}

pub fn main() {
    pretty_env_logger::init();

    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(UiPlugin)
        .add_system(update_counter.system())
        .add_startup_system(startup.system())
        .run();
}

fn startup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn_bundle(UiBundle {
        ui: Ui::new(Counter {
            value: 0,
            state: Default::default(),
        }),
        draw: Default::default(),
        stylesheet: assets.load("style.pwss"),
    });
}
