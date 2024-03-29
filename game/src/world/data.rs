use crate::world::explosion;
pub use crate::world::spatial::{Layer, Location};
use entity_table::{declare_entity_module, Entity};
use rand::{
    seq::{IteratorRandom, SliceRandom},
    Rng,
};
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
        oxygen: Meter,
        food: Meter,
        poison: Meter,
        radiation: Meter,
        power: Meter,
        satiation: Meter,
        destructible: (),
        to_remove: (),
        explodes_on_death: (),
        npc_type: NpcType,
        item: Item,
        money_item: u32,
        inventory: Inventory,
        money: u32,
        leaves_corpse: (),
        corpse: (),
        resurrects_in: Meter,
        simple_inventory: Vec<Entity>,
        get_on_touch: (),
        organs: Organs,
        simple_organs: Vec<Organ>,
        gun: Gun,
        hands: Hands,
        spread_poison: (),
        split_on_damage: (),
        floor_poison: (),
        bump_damage: RangeInclusive<u32>,
        radioactive: (),
        smoke: (),
        organ_clinic: (),
        shop: Shop,
        slow: u64,
        boss: (),
        tentacle: (),
    }
}
pub use components::{Components, EntityData, EntityUpdate};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Tile {
    Player,
    DeadPlayer,
    Floor,
    FloorBloody,
    FloorPoison,
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
    Snatcher,
    Poisoner,
    Divider,
    Glower,
    Venter,
    Corruptor,
    GunStore,
    ItemStore,
    OrganTrader,
    OrganClinic,
    Money(u32),
    Item(Item),
    Corpse(NpcType),
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
    pub fn clear(&mut self) {
        self.current = 0;
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
    Thief,
    Neutral,
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

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NpcType {
    Zombie,
    Climber,
    Trespasser,
    Snatcher,
    Boomer,
    Poisoner,
    Divider,
    Glower,
    Venter,
    Corruptor,
    GunStore,
    ItemStore,
    OrganTrader,
    OrganClinic,
}

impl NpcType {
    pub fn tile(self) -> Tile {
        match self {
            Self::Zombie => Tile::Zombie,
            Self::Climber => Tile::Climber,
            Self::Trespasser => Tile::Trespasser,
            Self::Boomer => Tile::Boomer,
            Self::Snatcher => Tile::Snatcher,
            Self::Poisoner => Tile::Poisoner,
            Self::Divider => Tile::Divider,
            Self::Glower => Tile::Glower,
            Self::Venter => Tile::Venter,
            Self::Corruptor => Tile::Corruptor,
            Self::GunStore => Tile::Corruptor,
            Self::ItemStore => Tile::ItemStore,
            Self::OrganTrader => Tile::OrganTrader,
            Self::OrganClinic => Tile::OrganClinic,
        }
    }
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
    const ALL: &'static [OrganTrait] = &[
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

    pub fn none() -> Self {
        Self {
            prolific: false,
            vampiric: false,
            radioactitve: false,
            damaged: false,
            embedded: false,
            transient: false,
        }
    }

    pub fn with_one_random<R: Rng>(rng: &mut R) -> Self {
        let mut s = Self::none();
        let trait_ = s.get_mut(OrganTrait::choose(rng));
        *trait_ = true;
        s
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
    Claw,
    CorruptedHeart,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Organ {
    pub type_: OrganType,
    pub traits: OrganTraits,
    pub cybernetic: bool,
    pub original: bool,
}

impl Organ {
    pub fn player_buy_price(&self) -> u32 {
        let mut price = match self.type_ {
            OrganType::Heart => 40,
            OrganType::Liver => 20,
            OrganType::Lung => 20,
            OrganType::Stomach => 20,
            OrganType::Appendix => 0,
            OrganType::Tumour => 0,
            OrganType::CronenbergPistol => 30,
            OrganType::CronenbergShotgun => 60,
            OrganType::CyberCore => 50,
            OrganType::Claw => 30,
            OrganType::CorruptedHeart => 3000,
        };
        if self.cybernetic {
            price *= 2;
        }
        let traits = self.traits.traits();
        if traits.len() == 1 {
            price = price * 2 / 3;
        } else if traits.len() > 1 {
            price = price / 3;
        }
        price
    }
    pub fn remove_price(&self) -> i32 {
        let traits = self.traits.traits();
        if self.original && traits.is_empty() {
            match self.type_ {
                OrganType::Heart => return -40,
                OrganType::Liver => return -20,
                OrganType::Lung => return -20,
                OrganType::Stomach => return -20,
                _ => (),
            }
        }
        if self.traits.embedded {
            80
        } else {
            20
        }
    }
    pub fn container_install_cost(&self) -> u32 {
        20
    }
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
    Shotgun,
    Pistol,
    RocketLauncher,
    ShotgunAmmo,
    PistolAmmo,
    Rocket,
}

impl Item {
    pub fn price(&self) -> u32 {
        match self {
            Self::Stimpack => 10,
            Self::Antidote => 5,
            Self::BloodVialEmpty => 10,
            Self::BloodVialFull => 15,
            Self::Battery => 30,
            Self::Food => 5,
            Self::AntiRads => 10,
            Self::OrganContainer(None) => 20,
            Self::OrganContainer(Some(_)) => 200,
            Self::Pistol => 30,
            Self::PistolAmmo => 10,
            Self::Shotgun => 40,
            Self::ShotgunAmmo => 10,
            Self::RocketLauncher => 80,
            Self::Rocket => 20,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Inventory {
    pub items: Vec<Option<Entity>>,
}

impl Inventory {
    pub fn new(size: usize) -> Self {
        Self {
            items: (0..size).map(|_| None).collect(),
        }
    }

    pub fn first_free_slot(&mut self) -> Option<&mut Option<Entity>> {
        for entry in self.items.iter_mut() {
            if entry.is_none() {
                return Some(entry);
            }
        }
        None
    }

    pub fn size(&self) -> usize {
        self.items.len()
    }

    pub fn get(&self, i: usize) -> Option<Entity> {
        self.items[i]
    }

    pub fn remove(&mut self, i: usize) -> Option<Entity> {
        use std::mem;
        mem::replace(&mut self.items[i], None)
    }

    pub fn items(&self) -> &[Option<Entity>] {
        &self.items
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Organs {
    organs: Vec<Option<Organ>>,
}

impl Organs {
    pub fn new(size: usize) -> Self {
        Self {
            organs: (0..size).map(|_| None).collect(),
        }
    }

    pub fn first_free_slot(&mut self) -> Option<&mut Option<Organ>> {
        for entry in self.organs.iter_mut() {
            if entry.is_none() {
                return Some(entry);
            }
        }
        None
    }

    pub fn get_slot_mut(&mut self, i: usize) -> &mut Option<Organ> {
        &mut self.organs[i]
    }

    pub fn get_mut(&mut self, i: usize) -> Option<&mut Organ> {
        self.organs[i].as_mut()
    }

    pub fn remove(&mut self, i: usize) -> Option<Organ> {
        use std::mem;
        mem::replace(&mut self.organs[i], None)
    }

    pub fn num_claws(&self) -> usize {
        let mut count = 0;
        for slot in self.organs.iter() {
            if let Some(organ) = slot.as_ref() {
                if organ.type_ == OrganType::Claw {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn organs(&self) -> &[Option<Organ>] {
        &self.organs
    }

    pub fn num_free_slots(&self) -> usize {
        self.organs.iter().filter(|s| s.is_none()).count()
    }

    pub fn choose_mut<R: Rng>(&mut self, rng: &mut R) -> Option<&mut Organ> {
        let index = (0..self.organs.len())
            .filter(|&i| self.organs[i].is_some())
            .choose(rng);
        index.map(|i| self.organs[i].as_mut().unwrap())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GunType {
    Pistol,
    Shotgun,
    RocketLauncher,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Gun {
    pub type_: GunType,
    pub ammo: Meter,
    pub hands_required: usize,
}

impl Gun {
    pub fn pistol() -> Self {
        Self {
            type_: GunType::Pistol,
            ammo: Meter::new_full(12),
            hands_required: 1,
        }
    }
    pub fn shotgun() -> Self {
        Self {
            type_: GunType::Shotgun,
            ammo: Meter::new_full(3),
            hands_required: 2,
        }
    }
    pub fn rocket_launcher() -> Self {
        Self {
            type_: GunType::RocketLauncher,
            ammo: Meter::new_full(1),
            hands_required: 2,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    Empty,
    Claw,
    Holding(Entity),
}

impl Hand {
    pub fn holding(&self) -> Option<Entity> {
        if let Hand::Holding(e) = self {
            Some(*e)
        } else {
            None
        }
    }

    pub fn is_holding(&self) -> bool {
        if let Hand::Holding(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_claw(&self) -> bool {
        if let Hand::Claw = self {
            true
        } else {
            false
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Hands {
    pub left: Hand,
    pub right: Hand,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Shop {
    pub message: String,
}
