use crate::{
    world::{
        data::*,
        explosion,
        spatial::{Layer, Layers, Location},
    },
    ExternalEvent, Message, World,
};
use coord_2d::Coord;
use direction::Direction;
use entity_table::Entity;
use rand::{seq::SliceRandom, Rng};

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
                    if let Some(projectile_damage) = self
                        .components
                        .projectile_damage
                        .get(projectile_entity)
                        .cloned()
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
        projectile_damage: ProjectileDamage,
        _projectile_movement_direction: Direction,
        entity_to_damage: Entity,
        rng: &mut R,
        external_events: &mut Vec<ExternalEvent>,
        message_log: &mut Vec<Message>,
    ) {
        self.damage_character(
            entity_to_damage,
            rng.gen_range(projectile_damage.hit_points),
            rng,
            external_events,
            message_log,
        );
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
        if let Some(&npc_type) = self.components.npc_type.get(character) {
            message_log.push(Message::NpcHit {
                npc_type,
                damage: hit_points_to_lose,
            });
        }
        if self.components.shop.contains(character) {
            if let Some(npc) = self.components.npc.get_mut(character) {
                npc.disposition = Disposition::Hostile;
                if let Some(npc_type) = self.components.npc_type.get_mut(character) {
                    message_log.push(Message::BecomesHostile(*npc_type));
                }
            }
            self.components.shop.remove(character);
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
            let hp_copy = hit_points.clone();
            hit_points.decrease(hit_points_to_lose);
            if self.components.split_on_damage.contains(character) {
                if hit_points.current() > 1 {
                    if let Some(coord) = self.spatial_table.coord_of(character) {
                        let copy_hit_poinst = hit_points.current() / 2;
                        hit_points.decrease(copy_hit_poinst);
                        let hit_points = hit_points.clone();
                        let mut copy_data = self.components.clone_entity_data(character);
                        copy_data.health = Some(Meter::new(copy_hit_poinst, hit_points.max()));
                        if let Some(copy_coord) = self.nearest_characterless_coord(coord) {
                            let copy_entity = self.entity_allocator.alloc();
                            self.components.insert_entity_data(copy_entity, copy_data);
                            let _ = self.spatial_table.update(
                                copy_entity,
                                Location {
                                    coord: copy_coord,
                                    layer: Some(Layer::Character),
                                },
                            );
                        }
                    }
                }
            }
            if self.components.boss.contains(character) {
                let hit_points = self.components.health.get(character).unwrap();
                let thresh1 = 2 * hp_copy.max() / 3;
                let thresh2 = 1 * hp_copy.max() / 3;
                if (hp_copy.current() > thresh1 && hit_points.current() <= thresh1)
                    || (hp_copy.current() > thresh2 && hit_points.current() <= thresh2)
                {
                    if let Some(coord) = self.random_characterless_coord(rng) {
                        let result = self.spatial_table.update(
                            character,
                            Location {
                                coord,
                                layer: Some(Layer::Character),
                            },
                        );
                        if result.is_ok() {
                            message_log.push(Message::CorruptorTeleport);
                        }
                    }
                }
            }
        }
    }

    pub fn damage_player<R: Rng>(
        &mut self,
        character: Entity,
        hit_points_to_lose: u32,
        rng: &mut R,
        external_events: &mut Vec<ExternalEvent>,
        message_log: &mut Vec<Message>,
    ) {
        let player_entity = self.components.player.entities().next().unwrap();
        if self.components.to_remove.contains(player_entity) {
            // prevent cascading damage on explosions
            return;
        }
        if let Some(&npc_type) = self.components.npc_type.get(character) {
            message_log.push(Message::PlayerHit {
                attacker_npc_type: npc_type,
                damage: hit_points_to_lose,
            });
        }
        let hit_points = self
            .components
            .health
            .get_mut(player_entity)
            .expect("character lacks hit_points");
        if hit_points_to_lose >= hit_points.current() {
            hit_points.set_current(0);
            self.character_die(player_entity, rng, external_events, message_log);
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
        if let Some(&npc_type) = self.components.npc_type.get(character) {
            message_log.push(Message::NpcDies(npc_type));
            self.components.corpse.insert(character, ());
            self.components.character.remove(character);
            self.components
                .tile
                .insert(character, Tile::Corpse(npc_type));
            let current_coord = self.spatial_table.coord_of(character).unwrap();
            if let Some(coord) = self.nearest_itemless_coord(current_coord) {
                let _ = self.spatial_table.update(
                    character,
                    Location {
                        coord,
                        layer: Some(Layer::Item),
                    },
                );
            } else {
                self.components.to_remove.insert(character, ());
            }
        } else {
            self.components.to_remove.insert(character, ());
        }
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
        if let Some(simple_inventory) = self.components.simple_inventory.get_mut(character) {
            use std::mem;
            let simple_inventory = mem::replace(simple_inventory, Vec::new());
            let current_coord = self.spatial_table.coord_of(character).unwrap();
            for entity in simple_inventory {
                if let Some(coord) = self.nearest_itemless_coord(current_coord) {
                    let _ = self.spatial_table.update(
                        entity,
                        Location {
                            coord,
                            layer: Some(Layer::Item),
                        },
                    );
                }
            }
        }
    }

    fn resurrect(&mut self, entity: Entity) {
        let current_coord = self.spatial_table.coord_of(entity).unwrap();
        if let Some(coord) = self.nearest_characterless_coord(current_coord) {
            let _ = self.spatial_table.update(
                entity,
                Location {
                    coord,
                    layer: Some(Layer::Character),
                },
            );
        } else {
            return;
        }
        if let Some(resurrects_in) = self.components.resurrects_in.get_mut(entity) {
            resurrects_in.set_current(resurrects_in.max());
        }
        if let Some(health) = self.components.health.get_mut(entity) {
            health.set_current(health.max());
        }
        self.components.corpse.remove(entity);
        self.components.character.insert(entity, ());
        if let Some(&Tile::Corpse(npc_type)) = self.components.tile.get(entity) {
            self.components.tile.insert(entity, npc_type.tile());
        }
    }

    pub fn handle_resurrection(&mut self) {
        let mut to_resurrect = Vec::new();
        for (entity, resurrects_in) in self.components.resurrects_in.iter_mut() {
            if let Some(health) = self.components.health.get(entity) {
                if health.current() == 0 {
                    if resurrects_in.current() == 0 {
                        to_resurrect.push(entity);
                    } else {
                        resurrects_in.decrease(1);
                    }
                }
            }
        }
        for entity in to_resurrect {
            self.resurrect(entity);
        }
    }

    pub fn handle_get_on_touch(&mut self) {
        for entity in self.components.get_on_touch.entities() {
            if self.components.character.contains(entity) {
                if let Some(simple_inventory) = self.components.simple_inventory.get_mut(entity) {
                    if let Some(coord) = self.spatial_table.coord_of(entity) {
                        if let Some(Layers {
                            item: Some(item_entity),
                            ..
                        }) = self.spatial_table.layers_at(coord).cloned()
                        {
                            self.spatial_table.remove(item_entity);
                            simple_inventory.push(item_entity);
                        }
                    }
                }
            }
        }
    }

    pub fn handle_spread_poison(&mut self) {
        for entity in self.components.spread_poison.entities() {
            if self.components.character.contains(entity) {
                if let Some(coord) = self.spatial_table.coord_of(entity) {
                    if let Some(coord) = self.nearest_non_poison_coord(coord) {
                        if let Some(Layers {
                            floor: Some(floor), ..
                        }) = self.spatial_table.layers_at(coord)
                        {
                            self.components.tile.insert(*floor, Tile::FloorPoison);
                            self.components.floor_poison.insert(*floor, ());
                        }
                    }
                }
            }
        }
    }

    pub fn add_player_initial_items(&mut self) {
        let entities = vec![
            self.spawn_item_no_coord(Item::PistolAmmo),
            self.spawn_item_no_coord(Item::AntiRads),
        ];
        let player = self.components.player.entities().next().unwrap();
        let inventory = self.components.inventory.get_mut(player).unwrap();
        for entity in entities {
            *inventory.first_free_slot().unwrap() = Some(entity);
        }
        let pistol = self.spawn_item_no_coord(Item::Pistol);
        self.components.hands.get_mut(player).unwrap().left = Hand::Holding(pistol);
    }

    pub fn make_floor_bloody(&mut self, coord: Coord) {
        if let Some(Layers {
            floor: Some(floor_entity),
            ..
        }) = self.spatial_table.layers_at(coord)
        {
            // XXX this changes non-floor tiles (road, footpath, etc) into floor tiles
            self.components
                .tile
                .insert(*floor_entity, Tile::FloorBloody);
        }
    }

    pub fn player_bump_combat<R: Rng>(
        &mut self,
        character: Entity,
        rng: &mut R,
        external_events: &mut Vec<ExternalEvent>,
        message_log: &mut Vec<Message>,
    ) {
        let mut damage = rng.gen_range(1..=2);
        for organ in self.active_player_organs() {
            if organ.type_ == OrganType::Claw {
                let mut mult = 1;
                if organ.cybernetic {
                    mult *= 2;
                }
                if organ.traits.damaged {
                    mult /= 2;
                }
                damage += rng.gen_range((2 * mult)..=(4 * mult));
            }
        }
        external_events.push(ExternalEvent::Melee);
        self.damage_character(character, damage, rng, external_events, message_log);
    }

    pub fn handle_player_organ_traits<R: Rng>(
        &mut self,
        rng: &mut R,
        message_log: &mut Vec<Message>,
    ) {
        let player_entity = self.components.player.entities().next().unwrap();
        let organs = self.components.organs.get(player_entity).unwrap().clone();
        for (i, organ) in organs.organs().into_iter().enumerate() {
            if let Some(organ) = organ {
                if organ.traits.radioactitve {
                    if rng.gen::<f64>() < 0.5 {
                        message_log.push(Message::IrradiatedByOrgan(*organ));
                        self.components
                            .radiation
                            .get_mut(player_entity)
                            .unwrap()
                            .increase(1);
                    }
                }
                if organ.traits.prolific {
                    if rng.gen::<f64>() < 0.02 {
                        if let Some(slot) = self
                            .components
                            .organs
                            .get_mut(player_entity)
                            .unwrap()
                            .first_free_slot()
                        {
                            message_log.push(Message::OrganDuplication(*organ));
                            *slot = Some(*organ);
                        }
                    }
                }
                if organ.traits.transient {
                    if rng.gen::<f64>() < 0.01 {
                        message_log.push(Message::OrganDisappear(*organ));
                        self.components
                            .organs
                            .get_mut(player_entity)
                            .unwrap()
                            .remove(i);
                    }
                }
            }
        }
    }

    pub fn handle_full_poison<R: Rng>(&mut self, rng: &mut R, message_log: &mut Vec<Message>) {
        let player_entity = self.components.player.entities().next().unwrap();
        let poison = self.components.poison.get_mut(player_entity).unwrap();
        if poison.is_full() {
            poison.clear();
            let organs = self.components.organs.get_mut(player_entity).unwrap();
            let mut non_damaged_indices = Vec::new();
            let mut damaged_indices = Vec::new();
            for (i, slot) in organs.organs().into_iter().enumerate() {
                if let Some(organ) = slot {
                    if organ.traits.damaged {
                        damaged_indices.push(i);
                    } else {
                        non_damaged_indices.push(i);
                    }
                }
            }
            non_damaged_indices.shuffle(rng);
            damaged_indices.shuffle(rng);
            if let Some(i) = non_damaged_indices.first() {
                if let Some(organ) = organs.get_mut(*i) {
                    message_log.push(Message::OrganDamagedByPoison(*organ));
                    organ.traits.damaged = true;
                }
            } else if let Some(i) = damaged_indices.first() {
                let organ = organs.remove(*i).unwrap();
                message_log.push(Message::OrganDestroyedByPoison(organ));
            }
        }
    }

    pub fn handle_full_radiation<R: Rng>(&mut self, rng: &mut R, message_log: &mut Vec<Message>) {
        let player_entity = self.components.player.entities().next().unwrap();
        let radiation = self.components.radiation.get_mut(player_entity).unwrap();
        if radiation.is_full() {
            radiation.clear();
            let organs = self.components.organs.get_mut(player_entity).unwrap();
            if organs.num_free_slots() > 0 {
                if rng.gen::<f64>() < 0.1 {
                    message_log.push(Message::GrowTumor);
                    *organs.first_free_slot().unwrap() = Some(Organ {
                        type_: OrganType::Tumour,
                        original: false,
                        cybernetic: false,
                        traits: OrganTraits {
                            prolific: true,
                            ..OrganTraits::none()
                        },
                    });
                    return;
                }
            }
            if let Some(organ_to_mutate) = organs.choose_mut(rng) {
                let trait_ = OrganTrait::choose(rng);
                let trait_value = organ_to_mutate.traits.get_mut(trait_);
                *trait_value = !*trait_value;
                if *trait_value {
                    message_log.push(Message::OrganGainsTrait {
                        organ: *organ_to_mutate,
                        trait_,
                    });
                } else {
                    message_log.push(Message::OrganLosesTrait {
                        organ: *organ_to_mutate,
                        trait_,
                    });
                }
            }
        }
    }

    pub fn handle_player_organs<R: Rng>(&mut self, rng: &mut R, message_log: &mut Vec<Message>) {
        let player_entity = self.components.player.entities().next().unwrap();
        self.components
            .satiation
            .get_mut(player_entity)
            .unwrap()
            .decrease(1);
        self.components
            .power
            .get_mut(player_entity)
            .unwrap()
            .decrease(1);
        if rng.gen::<f64>() < 0.1 {
            self.components
                .radiation
                .get_mut(player_entity)
                .unwrap()
                .increase(1);
            message_log.push(Message::AmbientRadiation);
        }
        let organs = self.active_player_organs();
        let mut max_health = 0;
        let mut max_power = 0;
        let mut oxygen_increase = -1;
        let mut num_claws = 0;
        for organ in &organs {
            match organ.type_ {
                OrganType::Claw => num_claws += 1,
                OrganType::Heart => {
                    let mut amount = 10;
                    if organ.cybernetic {
                        amount *= 2;
                    }
                    if organ.traits.damaged {
                        amount /= 2;
                    }
                    max_health += amount;
                }
                OrganType::CyberCore => {
                    let mut amount = 100;
                    if organ.cybernetic {
                        amount *= 2;
                    }
                    if organ.traits.damaged {
                        amount /= 2;
                    }
                    max_power += amount;
                }
                OrganType::Liver => {
                    let mut chance = 0.5;
                    if organ.cybernetic {
                        chance *= 2.0;
                    }
                    if organ.traits.damaged {
                        chance *= 0.5;
                    }
                    if rng.gen::<f64>() < chance {
                        self.components
                            .poison
                            .get_mut(player_entity)
                            .unwrap()
                            .decrease(1);
                    }
                }
                OrganType::CorruptedHeart => {
                    let mut amount = 50;
                    if organ.cybernetic {
                        amount *= 2;
                    }
                    if organ.traits.damaged {
                        amount /= 2;
                    }
                    max_health += amount;
                }
                OrganType::Lung => {
                    let mut amount = if organ.cybernetic { 2 } else { 1 };
                    if organ.traits.damaged {
                        if rng.gen::<f64>() < 0.5 {
                            amount = 0;
                        }
                    }
                    oxygen_increase += amount;
                }
                _ => (),
            }
        }
        self.components
            .health
            .get_mut(player_entity)
            .unwrap()
            .set_max(max_health);
        self.components
            .power
            .get_mut(player_entity)
            .unwrap()
            .set_max(max_power);
        if oxygen_increase > 0 {
            self.components
                .oxygen
                .get_mut(player_entity)
                .unwrap()
                .increase(oxygen_increase as u32);
        } else {
            self.components
                .oxygen
                .get_mut(player_entity)
                .unwrap()
                .decrease((-oxygen_increase) as u32);
        }
        // separate loop so stomach is applied after heart
        for organ in &organs {
            match organ.type_ {
                OrganType::Stomach => {
                    if rng.gen::<f64>() < 0.1 {
                        let food = self.components.food.get_mut(player_entity).unwrap();
                        if food.current() > 0 {
                            food.decrease(1);
                            let mut health_increase = 2;
                            if organ.cybernetic {
                                health_increase *= 2;
                            }
                            if organ.traits.damaged {
                                health_increase /= 2;
                            }
                            let health = self.components.health.get_mut(player_entity).unwrap();
                            if health.is_full() {
                                message_log.push(Message::DigestFoodNoHealthIncrease);
                            } else {
                                health.increase(health_increase);
                                message_log.push(Message::DigestFood {
                                    health_gain: health_increase,
                                });
                            }
                        } else {
                            let health = self.components.health.get_mut(player_entity).unwrap();
                            health.decrease(1);
                            message_log.push(Message::HungerDamage);
                        }
                    }
                }
                _ => (),
            }
        }
        if let Some(player_coord) = self.spatial_table.coord_of(player_entity) {
            if num_claws > 0 {
                // TODO avoid needing to bind `hands` twice here
                let hands = self.components.hands.get(player_entity).unwrap();
                if let Some(item_entity) = hands.left.holding() {
                    self.drop_item(item_entity, player_coord);
                    if let Some(item) = self.components.item.get(item_entity) {
                        message_log.push(Message::ClawDrop(*item));
                    }
                }
                let hands = self.components.hands.get_mut(player_entity).unwrap();
                hands.left = Hand::Claw;
            }
            if num_claws > 1 {
                // TODO avoid needing to bind `hands` twice here
                let hands = self.components.hands.get(player_entity).unwrap();
                if let Some(item_entity) = hands.right.holding() {
                    self.drop_item(item_entity, player_coord);
                    if let Some(item) = self.components.item.get(item_entity) {
                        message_log.push(Message::ClawDrop(*item));
                    }
                }
                let hands = self.components.hands.get_mut(player_entity).unwrap();
                hands.right = Hand::Claw;
            }
        }
    }

    fn drop_item(&mut self, item_entity: Entity, coord: Coord) {
        if let Some(coord) = self.nearest_itemless_coord(coord) {
            let _ = self.spatial_table.update(
                item_entity,
                Location {
                    coord,
                    layer: Some(Layer::Item),
                },
            );
        }
    }

    pub fn handle_poison(&mut self, message_log: &mut Vec<Message>) {
        for (entity, poison) in self.components.poison.iter_mut() {
            if let Some(coord) = self.spatial_table.coord_of(entity) {
                if let Some(Layers {
                    floor: Some(floor), ..
                }) = self.spatial_table.layers_at(coord)
                {
                    if self.components.floor_poison.contains(*floor) {
                        poison.increase(2);
                        message_log.push(Message::Poison);
                    }
                }
            }
        }
    }

    pub fn handle_asphyxiation(&mut self, message_log: &mut Vec<Message>) {
        for (entity, oxygen) in self.components.oxygen.iter() {
            if oxygen.current() == 0 {
                if let Some(health) = self.components.health.get_mut(entity) {
                    health.decrease(1);
                }
                message_log.push(Message::LackOfOxygen);
            }
        }
    }

    pub fn handle_smoke(&mut self, message_log: &mut Vec<Message>) {
        let oxygen_entities = self.components.oxygen.entities().collect::<Vec<_>>();
        for entity in oxygen_entities {
            if let Some(coord) = self.spatial_table.coord_of(entity) {
                for smoke_entity in self.components.smoke.entities() {
                    if let Some(smoke_coord) = self.spatial_table.coord_of(smoke_entity) {
                        if let Some(distance) =
                            self.line_distance_stopping_at_solid(coord, smoke_coord)
                        {
                            if distance < 4 {
                                let oxygen = self.components.oxygen.get_mut(entity).unwrap();
                                oxygen.decrease(2);
                                message_log.push(Message::Smoke);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn handle_radiation(&mut self, message_log: &mut Vec<Message>) {
        let radiation_entities = self.components.radiation.entities().collect::<Vec<_>>();
        for entity in radiation_entities {
            if let Some(coord) = self.spatial_table.coord_of(entity) {
                for r_entity in self.components.radioactive.entities() {
                    if let Some(r_coord) = self.spatial_table.coord_of(r_entity) {
                        if let Some(distance) = self.line_distance_stopping_at_solid(coord, r_coord)
                        {
                            let radiation = self.components.radiation.get_mut(entity).unwrap();
                            if distance < 5 {
                                radiation.increase(2);
                                message_log.push(Message::RadiationVeryClose);
                            } else if distance < 10 {
                                message_log.push(Message::RadiationClose);
                                radiation.increase(1);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn remove_corrpution(&mut self) {
        let to_remove = self.components.tentacle.entities().collect::<Vec<_>>();
        for entity in to_remove {
            self.remove_entity(entity);
        }
        for edge in self.spatial_table.grid_size().edge_iter() {
            let layers = self.spatial_table.layers_at_checked(edge);
            if layers.feature.is_none() {
                self.spawn_debris(edge);
            }
        }
    }
}
