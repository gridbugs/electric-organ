use crate::world::explosion;
pub use crate::world::spatial::{Layer, Location};
use entity_table::declare_entity_module;
use rand::{seq::SliceRandom, Rng};
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
        item: Item,
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
    Money,
    Item(Item),
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

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OrganTrait {
    Prolific,
    Vampiric,
    Radioactitve,
    Damaged,
    Embedded,
    Transient,
}

impl OrganTrait {
    const ALL: &[OrganTrait] = &[
        OrganTrait::Prolific,
        OrganTrait::Vampiric,
        OrganTrait::Radioactitve,
        OrganTrait::Damaged,
        OrganTrait::Embedded,
        OrganTrait::Transient,
    ];
    pub fn choose<R: Rng>(rng: &mut R) -> Self {
        *Self::ALL.choose(rng).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OrganTraits {
    pub prolific: bool,
    pub vampiric: bool,
    pub radioactitve: bool,
    pub damaged: bool,
    pub embedded: bool,
    pub transient: bool,
}

impl OrganTraits {
    pub fn traits(&self) -> Vec<OrganTrait> {
        let mut ret = Vec::new();
        if self.prolific {
            ret.push(OrganTrait::Prolific);
        }
        if self.vampiric {
            ret.push(OrganTrait::Vampiric);
        }
        if self.radioactitve {
            ret.push(OrganTrait::Radioactitve);
        }
        if self.damaged {
            ret.push(OrganTrait::Damaged);
        }
        if self.embedded {
            ret.push(OrganTrait::Embedded);
        }
        if self.transient {
            ret.push(OrganTrait::Transient);
        }
        ret
    }

    pub fn get_mut(&mut self, trait_: OrganTrait) -> &mut bool {
        match trait_ {
            OrganTrait::Prolific => &mut self.prolific,
            OrganTrait::Vampiric => &mut self.vampiric,
            OrganTrait::Radioactitve => &mut self.radioactitve,
            OrganTrait::Damaged => &mut self.damaged,
            OrganTrait::Embedded => &mut self.embedded,
            OrganTrait::Transient => &mut self.transient,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OrganType {
    Heart,
    Liver,
    Lung,
    Stomach,
    Appendix,
    Tumour,
    CronenbergPistol,
    CronenbergShotgun,
    CyberCore,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Organ {
    pub type_: OrganType,
    pub traits: OrganTraits,
    pub cybernetic: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    Stimpack,
    Antidote,
    BloodVialEmpty,
    BloodVialFull,
    Battery,
    Food,
    AntiRads,
    OrganContainer(Option<Organ>),
}
