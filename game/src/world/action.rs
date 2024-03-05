use crate::{
    world::{
        data::{CollidesWith, OnCollision, ProjectileDamage},
        explosion,
    },
    ExternalEvent, Message, World,
};
use direction::Direction;
use entity_table::Entity;
use rand::Rng;

impl World {
    pub fn projectile_move<R: Rng>(
        &mut self,
        projectile_entity: Entity,
        movement_direction: Direction,
        external_events: &mut Vec<ExternalEvent>,
        message_log: &mut Vec<Message>,
        rng: &mut R,
    ) {
        if let Some(current_coord) = self.spatial_table.coord_of(projectile_entity) {
            let next_coord = current_coord + movement_direction.coord();
            let collides_with = self
                .components
                .collides_with
                .get(projectile_entity)
                .cloned()
                .unwrap_or(CollidesWith {
                    solid: true,
                    character: false,
                });
            if let Some(&spatial_cell) = self.spatial_table.layers_at(next_coord) {
                if let Some(character_entity) = spatial_cell.character {
                    if let Some(&projectile_damage) =
                        self.components.projectile_damage.get(projectile_entity)
                    {
                        self.apply_projectile_damage(
                            projectile_entity,
                            projectile_damage,
                            movement_direction,
                            character_entity,
                            rng,
                            external_events,
                            message_log,
                        );
                    }
                }
                if let Some(entity_in_cell) = spatial_cell.feature.or(spatial_cell.character) {
                    let is_particle = self.components.particle.contains(projectile_entity);
                    let solid_collision = if collides_with.solid && is_particle {
                        self.components.solid_for_particles.contains(entity_in_cell)
                    } else {
                        self.components.solid.contains(entity_in_cell)
                    };
                    if solid_collision
                        || (collides_with.character
                            && self.components.character.contains(entity_in_cell))
                    {
                        self.projectile_stop(projectile_entity, external_events, message_log, rng);
                        return;
                    }
                }
                let _ignore_err = self
                    .spatial_table
                    .update_coord(projectile_entity, next_coord);
            } else {
                self.projectile_stop(projectile_entity, external_events, message_log, rng);
                return;
            }
        } else {
            self.components.remove_entity(projectile_entity);
            self.realtime_components.remove_entity(projectile_entity);
            self.spatial_table.remove(projectile_entity);
        }
    }

    fn apply_projectile_damage<R: Rng>(
        &mut self,
        _projectile_entity: Entity,
        _projectile_damage: ProjectileDamage,
        _projectile_movement_direction: Direction,
        _entity_to_damage: Entity,
        _rng: &mut R,
        _external_events: &mut Vec<ExternalEvent>,
        _message_log: &mut Vec<Message>,
    ) {
    }

    pub fn projectile_stop<R: Rng>(
        &mut self,
        projectile_entity: Entity,
        external_events: &mut Vec<ExternalEvent>,
        message_log: &mut Vec<Message>,
        rng: &mut R,
    ) {
        if let Some(current_coord) = self.spatial_table.coord_of(projectile_entity) {
            if let Some(on_collision) = self.components.on_collision.get(projectile_entity).cloned()
            {
                match on_collision {
                    OnCollision::Explode(explosion_spec) => {
                        explosion::explode(
                            self,
                            current_coord,
                            explosion_spec,
                            external_events,
                            message_log,
                            rng,
                        );
                        self.spatial_table.remove(projectile_entity);
                        self.components.remove_entity(projectile_entity);
                        self.entity_allocator.free(projectile_entity);
                        self.realtime_components.remove_entity(projectile_entity);
                    }

                    OnCollision::Remove => {
                        self.spatial_table.remove(projectile_entity);
                        self.components.remove_entity(projectile_entity);
                        self.entity_allocator.free(projectile_entity);
                        self.realtime_components.remove_entity(projectile_entity);
                    }
                    OnCollision::RemoveRealtime => {
                        self.realtime_components.remove_entity(projectile_entity);
                        self.components.realtime.remove(projectile_entity);
                        self.components.blocks_gameplay.remove(projectile_entity);
                    }
                }
            }
        }
        self.realtime_components.movement.remove(projectile_entity);
    }

    pub fn damage_character<R: Rng>(
        &mut self,
        character: Entity,
        hit_points_to_lose: u32,
        rng: &mut R,
        external_events: &mut Vec<ExternalEvent>,
        message_log: &mut Vec<Message>,
    ) {
        if self.components.to_remove.contains(character) {
            // prevent cascading damage on explosions
            return;
        }
        let hit_points = self
            .components
            .health
            .get_mut(character)
            .expect("character lacks hit_points");
        if hit_points_to_lose >= hit_points.current() {
            hit_points.set_current(0);
            self.character_die(character, rng, external_events, message_log);
        } else {
            hit_points.decrease(hit_points_to_lose);
        }
    }

    fn character_die<R: Rng>(
        &mut self,
        character: Entity,
        rng: &mut R,
        external_events: &mut Vec<ExternalEvent>,
        message_log: &mut Vec<Message>,
    ) {
        self.components.to_remove.insert(character, ());
        if self.components.explodes_on_death.contains(character) {
            if let Some(coord) = self.spatial_table.coord_of(character) {
                self.components.explodes_on_death.remove(character);
                use explosion::spec::*;
                let spec = Explosion {
                    mechanics: Mechanics { range: 2 },
                    particle_emitter: ParticleEmitter {
                        duration: Duration::from_millis(400),
                        num_particles_per_frame: 100,
                        min_step: Duration::from_millis(100),
                        max_step: Duration::from_millis(300),
                        fade_duration: Duration::from_millis(500),
                    },
                };
                explosion::explode(self, coord, spec, external_events, message_log, rng);
            }
        }
    }
}
