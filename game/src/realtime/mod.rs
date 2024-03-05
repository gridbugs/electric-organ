use crate::{world::World, ExternalEvent, Message};
use entity_table::Entity;
use entity_table_realtime::{
    self, declare_realtime_entity_module, ContextContainsRealtimeComponents,
};
use rand_isaac::Isaac64Rng;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub mod flicker;
use flicker::FlickerState;

pub mod fade;
use fade::FadeState;

pub mod light_colour_fade;
use light_colour_fade::LightColourFadeState;

pub mod movement;
use movement::MovementState;

pub mod particle;
use particle::ParticleEmitterState;

pub struct Context<'a> {
    world: &'a mut World,
    external_events: &'a mut Vec<ExternalEvent>,
    message_log: &'a mut Vec<Message>,
    rng: &'a mut Isaac64Rng,
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
        fade: FadeState,
        light_colour_fade: LightColourFadeState,
        movement: MovementState,
        particle_emitter: ParticleEmitterState,
    }
}
pub use components::RealtimeComponents;

pub const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / 60);

pub fn period_per_frame(num_per_frame: u32) -> Duration {
    FRAME_DURATION / num_per_frame
}

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
    pub fn tick(
        &mut self,
        world: &mut World,
        external_events: &mut Vec<ExternalEvent>,
        message_log: &mut Vec<Message>,
        rng: &mut Isaac64Rng,
    ) {
        self.realtime_entities
            .extend(world.components.realtime.entities());
        let mut context = Context {
            world,
            external_events,
            message_log,
            rng,
        };
        for entity in self.realtime_entities.drain(..) {
            entity_table_realtime::process_entity_frame(entity, FRAME_DURATION, &mut context);
        }
    }
}
