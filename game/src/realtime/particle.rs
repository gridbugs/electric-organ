use crate::{
    realtime::{
        fade::{FadeProgress, FadeState},
        light_colour_fade::LightColourFadeState,
        movement, Context,
    },
    world::data::Tile,
};
use entity_table_realtime::{
    Entity, RealtimeComponent, RealtimeComponentApplyEvent, ScheduledRealtimeComponent,
};
use rand::{Rng, SeedableRng};
use rand_isaac::Isaac64Rng;
pub use rgb_int::Rgba32;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vector::Radial;
pub type Light = visible_area_detection::Light<visible_area_detection::vision_distance::Circle>;

pub mod spec {
    pub use super::Light;
    pub use crate::world::data::Tile;
    pub use rand_range::{UniformInclusiveRange, UniformLeftInclusiveRange};
    pub use rational::Rational;
    pub use rgb_int::{Rgb24, Rgba32};
    use serde::{Deserialize, Serialize};
    pub use std::time::Duration;
    pub use vector::Radians;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Possible<T: Clone> {
        pub chance: Rational,
        pub value: T,
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct Damage {
        pub range: UniformInclusiveRange<u32>,
        pub push_back: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Movement {
        pub angle_range: UniformLeftInclusiveRange<Radians>,
        pub cardinal_period_range: UniformInclusiveRange<Duration>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LightColourFade {
        pub duration: Duration,
        pub from: Rgb24,
        pub to: Rgb24,
    }

    #[derive(Default, Debug, Clone, Serialize, Deserialize)]
    pub struct Particle {
        pub fade_duration: Option<Duration>,
        pub tile: Option<Tile>,
        pub movement: Option<Movement>,
        pub colour_hint: Option<UniformInclusiveRange<Rgba32>>,
        pub light_colour_fade: Option<LightColourFade>,
        pub possible_light: Option<Possible<Light>>,
        pub possible_particle_emitter: Option<Possible<Box<ParticleEmitter>>>,
        pub possible_damage: Option<Possible<Damage>>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ParticleEmitter {
        pub emit_particle_every_period: Duration,
        pub particle: Particle,
        pub fade_out_duration: Option<Duration>,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FadeOutState {
    total: Duration,
    elapsed: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleEmitterState {
    emit_particle_every_period: Duration,
    particle_spec: spec::Particle,
    fade_out_state: Option<FadeOutState>,
    rng: Isaac64Rng,
}

pub struct SpawnParticle {
    movement_state: Option<movement::MovementState>,
    fade_state: Option<FadeState>,
    tile: Option<Tile>,
    colour_hint: Option<Rgba32>,
    light_colour_fade_state: Option<LightColourFadeState>,
    light: Option<Light>,
    particle_emitter: Option<Box<ParticleEmitterState>>,
}

impl<T: Clone> spec::Possible<T> {
    fn choose<R: Rng>(&self, rng: &mut R) -> Option<T> {
        if self.chance.roll(rng) {
            Some(self.value.clone())
        } else {
            None
        }
    }
}

impl spec::Movement {
    fn choose<R: Rng>(&self, rng: &mut R) -> movement::MovementState {
        const VECTOR_LENGTH: f64 = 1000.;
        let angle = self.angle_range.choose(rng);
        let radial = Radial {
            angle,
            length: VECTOR_LENGTH,
        };
        movement::spec::Movement {
            path: radial.to_cartesian().to_coord_round_nearest(),
            repeat: movement::spec::Repeat::Forever,
            cardinal_step_duration: self.cardinal_period_range.choose(rng),
        }
        .build()
    }
}

impl spec::ParticleEmitter {
    pub fn build<R: Rng>(self, rng: &mut R) -> ParticleEmitterState {
        ParticleEmitterState {
            emit_particle_every_period: self.emit_particle_every_period,
            particle_spec: self.particle,
            fade_out_state: self.fade_out_duration.map(|d| FadeOutState {
                total: d,
                elapsed: Duration::from_millis(0),
            }),
            rng: Isaac64Rng::from_rng(rng).unwrap(),
        }
    }
}

impl FadeOutState {
    fn fade(&mut self, duration: Duration) -> FadeProgress {
        self.elapsed += duration;
        if self.elapsed > self.total {
            FadeProgress::Complete
        } else {
            let ratio = ((self.elapsed.as_nanos() * 256) / self.total.as_nanos()).min(255) as u8;
            FadeProgress::Fading(ratio)
        }
    }
}

impl RealtimeComponent for ParticleEmitterState {
    type Event = SpawnParticle;
    fn tick(&mut self) -> (Self::Event, Duration) {
        let until_next_event = self.emit_particle_every_period;
        let (fade_state, light_colour_fade_state) = match self.fade_out_state.as_mut() {
            None => (
                self.particle_spec.fade_duration.map(|d| FadeState::new(d)),
                self.particle_spec.light_colour_fade.as_ref().map(|l| {
                    let fade_state = FadeState::new(l.duration);
                    LightColourFadeState {
                        fade_state,
                        from: l.from,
                        to: l.to,
                    }
                }),
            ),
            Some(fade_out_state) => {
                let fade_out_progress = fade_out_state.fade(until_next_event);
                (
                    self.particle_spec
                        .fade_duration
                        .map(|d| FadeState::new_with_progress(d, fade_out_progress)),
                    self.particle_spec.light_colour_fade.as_ref().map(|l| {
                        let fade_state =
                            FadeState::new_with_progress(l.duration, fade_out_progress);
                        LightColourFadeState {
                            fade_state,
                            from: l.from,
                            to: l.to,
                        }
                    }),
                )
            }
        };
        let event = SpawnParticle {
            movement_state: self
                .particle_spec
                .movement
                .as_ref()
                .map(|m| m.choose(&mut self.rng)),
            fade_state,
            tile: self.particle_spec.tile,
            colour_hint: self
                .particle_spec
                .colour_hint
                .map(|c| c.choose(&mut self.rng)),
            light_colour_fade_state,
            light: self
                .particle_spec
                .possible_light
                .as_ref()
                .and_then(|l| l.choose(&mut self.rng)),
            particle_emitter: self
                .particle_spec
                .possible_particle_emitter
                .as_ref()
                .and_then(|p| {
                    p.choose(&mut self.rng)
                        .map(|p| Box::new(p.build(&mut self.rng)))
                }),
        };
        (event, until_next_event)
    }
}

impl<'a> RealtimeComponentApplyEvent<Context<'a>> for ParticleEmitterState {
    fn apply_event(mut spawn_particle: SpawnParticle, entity: Entity, context: &mut Context<'a>) {
        let coord = if let Some(coord) = context.world.spatial_table.coord_of(entity) {
            coord
        } else {
            return;
        };
        let particle_entity = context.world.entity_allocator.alloc();
        if let Some(movement) = spawn_particle.movement_state.take() {
            context
                .world
                .realtime_components
                .movement
                .insert_with_schedule(
                    particle_entity,
                    ScheduledRealtimeComponent {
                        until_next_tick: movement.cardinal_step_duration(),
                        component: movement,
                    },
                );
        }
        context
            .world
            .spatial_table
            .update_coord(particle_entity, coord)
            .unwrap();
        context
            .world
            .components
            .particle
            .insert(particle_entity, ());
        if let Some(tile) = spawn_particle.tile {
            context.world.components.tile.insert(particle_entity, tile);
        }
        if let Some(fade_state) = spawn_particle.fade_state {
            context
                .world
                .realtime_components
                .fade
                .insert(particle_entity, fade_state);
        }
        context
            .world
            .components
            .realtime
            .insert(particle_entity, ());
        if let Some(colour_hint) = spawn_particle.colour_hint {
            context
                .world
                .components
                .colour_hint
                .insert(particle_entity, colour_hint);
        }
        if let Some(light) = spawn_particle.light.take() {
            context
                .world
                .components
                .light
                .insert(particle_entity, light);
        }
        if let Some(light_colour_fade) = spawn_particle.light_colour_fade_state.take() {
            context
                .world
                .realtime_components
                .light_colour_fade
                .insert(particle_entity, light_colour_fade);
        }
        if let Some(particle_emitter) = spawn_particle.particle_emitter.take() {
            context
                .world
                .realtime_components
                .particle_emitter
                .insert(particle_entity, *particle_emitter);
        }
    }
}
