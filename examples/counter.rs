use bevy::prelude::*;
use bevy_pixel_widgets::prelude::*;
use bevy_pixel_widgets::widget;

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

    fn update(&mut self, message: Self::Message) -> Vec<Command<Message>> {
        match message {
            Message::UpPressed => {
                self.value += 1;
                println!("{}", self.value);
                Vec::new()
            }
            Message::DownPressed => {
                self.value -= 1;
                println!("{}", self.value);
                Vec::new()
            }
        }
    }

    fn view(&mut self) -> widget::Node<Message> {
        let mut state = self.state.tracker();
        widget::Column::new()
            .push(widget::Button::new(state.get("up"), widget::Text::new("Up")).on_clicked(Message::UpPressed))
            .push(widget::Text::new(format!("Count: {}", self.value)))
            .push(widget::Button::new(state.get("down"), widget::Text::new("Down")).on_clicked(Message::DownPressed))
            .into_node()
    }
}

pub fn main() {
    App::build()
        .add_default_plugins()
        .add_plugin(UiPlugin::<Counter>::default())
        .add_startup_system(startup.system())
        .run();
}

fn startup(mut commands: Commands) {
    UiComponents::new(Counter {
        value: 0,
        state: Default::default()
    }).spawn(&mut commands);

    commands.spawn(Camera2dComponents::default());
}