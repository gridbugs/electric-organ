use crate::world::World;
use entity_table::Entity;
use entity_table_realtime::{
    self, declare_realtime_entity_module, ContextContainsRealtimeComponents,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub mod flicker;
use flicker::FlickerState;

pub struct Context<'a> {
    world: &'a mut World,
}

impl<'a> ContextContainsRealtimeComponents for Context<'a> {
    type Components = RealtimeComponents;
    fn components_mut(&mut self) -> &mut Self::Components {
        &mut self.world.realtime_components
    }
    fn realtime_entities(&self) -> entity_table::Entities {
        self.world.components.realtime.entities()
    }
}

declare_realtime_entity_module! {
    components<'a>[Context<'a>] {
        flicker: FlickerState,
    }
}
pub use components::RealtimeComponents;

pub const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / 60);

#[derive(Default)]
pub struct AnimationContext {
    realtime_entities: Vec<Entity>,
}

impl Serialize for AnimationContext {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        ().serialize(s)
    }
}

impl<'a> Deserialize<'a> for AnimationContext {
    fn deserialize<D: serde::Deserializer<'a>>(d: D) -> Result<Self, D::Error> {
        let () = Deserialize::deserialize(d)?;
        Ok(Self::default())
    }
}

impl AnimationContext {
    pub fn tick(&mut self, world: &mut World) {
        self.realtime_entities
            .extend(world.components.realtime.entities());
        let mut context = Context { world };
        for entity in self.realtime_entities.drain(..) {
            entity_table_realtime::process_entity_frame(entity, FRAME_DURATION, &mut context);
        }
    }
}
