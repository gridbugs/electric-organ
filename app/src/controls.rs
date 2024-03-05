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
