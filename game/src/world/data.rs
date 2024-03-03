pub use crate::world::spatial::{Layer, Location};
use entity_table::declare_entity_module;
use rgb_int::Rgb24;
use serde::{Deserialize, Serialize};
use visible_area_detection::{vision_distance, Light};

declare_entity_module! {
    components {
        realtime: (),
        tile: Tile,
        solid: (),
        door_state: DoorState,
        opacity: u8,
        stairs_down: (),
        colour_hint: Rgb24,
        light: Light<vision_distance::Circle>,
    }
}
pub use components::{Components, EntityData, EntityUpdate};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Tile {
    Player,
    Floor,
    Wall,
    Street,
    Alley,
    Footpath,
    DoorClosed,
    DoorOpen,
    StairsDown,
    StairsUp,
    Debris,
    DebrisBurning,
    Tentacle,
    TentacleGlow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DoorState {
    Open,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meter {
    current: u32,
    max: u32,
}

impl Meter {
    pub fn new(current: u32, max: u32) -> Self {
        Self { current, max }
    }
    pub fn current_and_max(&self) -> (u32, u32) {
        (self.current, self.max)
    }
    pub fn current(&self) -> u32 {
        self.current
    }
    pub fn max(&self) -> u32 {
        self.max
    }
    pub fn set_current(&mut self, to: u32) {
        self.current = to.min(self.max);
    }
    pub fn decrease(&mut self, by: u32) {
        self.current = self.current.saturating_sub(by);
    }
    pub fn increase(&mut self, by: u32) {
        self.set_current(self.current + by);
    }
    pub fn set_max(&mut self, to: u32) {
        self.max = to;
        self.set_current(self.current);
    }
    pub fn is_empty(&self) -> bool {
        self.current == 0
    }
    pub fn is_full(&self) -> bool {
        self.current == self.max
    }
    pub fn fill(&mut self) {
        self.current = self.max;
    }
}
