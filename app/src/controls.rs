use chargrid::input::{Input, KeyboardInput};
use direction::CardinalDirection;
use maplit::btreemap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppInput {
    Direction(CardinalDirection),
    Wait,
    FireEquipped,
    FireBody,
    MessageLog,
    Get,
    DropItem,
    ApplyItem,
    UnequipItem,
    Reload,
    ViewOrgans,
}

#[derive(Serialize, Deserialize)]
pub struct Controls {
    keys: BTreeMap<KeyboardInput, AppInput>,
}

impl Default for Controls {
    fn default() -> Self {
        let keys = btreemap![
            KeyboardInput::Left => AppInput::Direction(CardinalDirection::West),
            KeyboardInput::Right => AppInput::Direction(CardinalDirection::East),
            KeyboardInput::Up => AppInput::Direction(CardinalDirection::North),
            KeyboardInput::Down => AppInput::Direction(CardinalDirection::South),
            KeyboardInput::Char(' ') => AppInput::Wait,
            KeyboardInput::Char('f') => AppInput::FireEquipped,
            KeyboardInput::Char('c') => AppInput::FireBody,
            KeyboardInput::Char('m') => AppInput::MessageLog,
            KeyboardInput::Char('g') => AppInput::Get,
            KeyboardInput::Char('d') => AppInput::DropItem,
            KeyboardInput::Char('a') => AppInput::ApplyItem,
            KeyboardInput::Char('u') => AppInput::UnequipItem,
            KeyboardInput::Char('r') => AppInput::Reload,
            KeyboardInput::Char('o') => AppInput::ViewOrgans,
        ];
        Self { keys }
    }
}
impl Controls {
    pub fn get(&self, input: Input) -> Option<AppInput> {
        match input {
            Input::Keyboard(keyboard_input) => self.keys.get(&keyboard_input).cloned(),
            Input::Mouse(_) => None,
        }
    }
}
