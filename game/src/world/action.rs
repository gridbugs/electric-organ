use crate::{
    world::data::{CollidesWith, OnCollision, ProjectileDamage},
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
        _external_events: &mut Vec<ExternalEvent>,
        _message_log: &mut Vec<Message>,
        _rng: &mut R,
    ) {
        if let Some(_current_coord) = self.spatial_table.coord_of(projectile_entity) {
            if let Some(on_collision) = self.components.on_collision.get(projectile_entity).cloned()
            {
                match on_collision {
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
}
