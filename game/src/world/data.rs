use crate::world::explosion;
pub use crate::world::spatial::{Layer, Location};
use entity_table::declare_entity_module;
use rgb_int::Rgba32;
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;
use visible_area_detection::{vision_distance, Light};

declare_entity_module! {
    components {
        player: (),
        realtime: (),
        blocks_gameplay: (),
        tile: Tile,
        solid: (),
        solid_for_particles: (),
        difficult: (),
        character: (),
        particle: (),
        door_state: DoorState,
        opacity: u8,
        stairs_down: (),
        stairs_up: (),
        exit: (),
        colour_hint: Rgba32,
        light: Light<vision_distance::Circle>,
        collides_with: CollidesWith,
        projectile_damage: ProjectileDamage,
        on_collision: OnCollision,
        npc: Npc,
        health: Meter,
        destructible: (),
        to_remove: (),
        explodes_on_death: (),
        npc_type: NpcType,
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
    Exit,
    Bullet,
    Zombie,
    Climber,
    Trespasser,
    Boomer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DoorState {
    Open,
    Closed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Meter {
    current: u32,
    max: u32,
}

impl Meter {
    pub fn new(current: u32, max: u32) -> Self {
        Self { current, max }
    }
    pub fn new_full(max: u32) -> Self {
        Self::new(max, max)
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CollidesWith {
    pub solid: bool,
    pub character: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectileDamage {
    pub hit_points: RangeInclusive<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OnCollision {
    Remove,
    RemoveRealtime,
    Explode(explosion::spec::Explosion),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Disposition {
    Hostile,
    Afraid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NpcMovement {
    pub can_traverse_difficult: bool,
    pub can_open_doors: bool,
}

impl NpcMovement {
    pub const ALL: &'static [Self] = &[
        NpcMovement {
            can_traverse_difficult: false,
            can_open_doors: false,
        },
        NpcMovement {
            can_traverse_difficult: false,
            can_open_doors: true,
        },
        NpcMovement {
            can_traverse_difficult: true,
            can_open_doors: false,
        },
        NpcMovement {
            can_traverse_difficult: true,
            can_open_doors: true,
        },
    ];
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Npc {
    pub disposition: Disposition,
    pub movement: NpcMovement,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcType {
    Zombie,
    Climber,
    Trespasser,
    Boomer,
}
