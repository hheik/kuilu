use std::collections::{
    hash_map::{Iter, IterMut},
    HashMap,
};

use bevy::ecs::prelude::SystemStage;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

mod chunk2d;
mod terrain_gen2d;
mod texel2d;
mod texel_behaviour2d;

pub use chunk2d::*;
pub use terrain_gen2d::*;
pub use texel2d::*;
pub use texel_behaviour2d::*;

use crate::util::{frame_counter::FrameCounter, math::*, Vector2I};

pub struct Terrain2DPlugin;

impl Plugin for Terrain2DPlugin {
    fn build(&self, app: &mut App) {
        // Add terrain stages
        app.add_stage_before(
            CoreStage::Update,
            TerrainStages::Simulation,
            SystemStage::parallel(),
        );
        // After update, but before rapier
        app.add_stage_before(
            PhysicsStages::SyncBackend,
            TerrainStages::EventHandler,
            SystemStage::parallel(),
        )
        .add_stage_after(
            TerrainStages::EventHandler,
            TerrainStages::ChunkSync,
            SystemStage::parallel(),
        );

        app.register_type::<TerrainChunk2D>()
            .insert_resource(Terrain2D::new(
                Some(Terrain2D::WORLD_HEIGHT),
                Some(0),
                Some(0),
                Some(Terrain2D::WORLD_WIDTH),
            ))
            .add_event::<TerrainEvent2D>()
            .add_system_to_stage(TerrainStages::Simulation, terrain_simulation)
            .add_system_to_stage(TerrainStages::EventHandler, emit_terrain_events)
            .add_system_to_stage(
                TerrainStages::EventHandler,
                // TODO: Figure out why .after() creates a lagspike for the first frame
                chunk_spawner.before(emit_terrain_events),
            )
            .add_system_to_stage(TerrainStages::ChunkSync, chunk_sprite_sync)
            .add_system_to_stage(CoreStage::PostUpdate, chunk_collision_sync);
    }
}

#[derive(StageLabel)]
pub enum TerrainStages {
    /// Terrain simulation stage. Should run before update.
    Simulation,
    /// The stage that Handles collected events and creates new chunk entities as needed. Should run after update.
    EventHandler,
    /// Chunk sync systems (e.g. collsion and sprite) run in this stage.
    ChunkSync,
}

fn terrain_simulation(
    mut terrain: ResMut<Terrain2D>,
    frame_counter: Res<FrameCounter>,
    mut debug_draw: ResMut<bevy_prototype_debug_lines::DebugLines>,
) {
    let simulation_frame = (frame_counter.frame % u8::MAX as u64) as u8 + 1;

    let indices = terrain
        .chunk_iter()
        .map(|(chunk_index, _)| *chunk_index)
        .collect::<Vec<Chunk2DIndex>>()
        .clone();

    for chunk_index in indices.iter() {
        // Mark few chunks dirty in interval. Should help activate stale chunks
        if let Some(chunk) = terrain.index_to_chunk_mut(&chunk_index) {
            let interval = 1;
            if frame_counter.frame % interval == 0 {
                let i = ((frame_counter.frame / interval) % 100) as i32;
                if (chunk_index.y % 10) * 10 + (chunk_index.x % 10) == i {
                    chunk.mark_all_dirty();
                }
            }
        };

        if let Some(rect) = &terrain
            .index_to_chunk(&chunk_index)
            .map_or(None, |chunk| chunk.dirty_rect.clone())
        {
            if let Some(chunk) = terrain.index_to_chunk_mut(&chunk_index) {
                chunk.mark_clean();
            } else {
                continue;
            };

            // Texel simulation
            let mut y_range: Vec<_> = (rect.min.y..rect.max.y + 1).collect();
            let mut x_range: Vec<_> = (rect.min.x..rect.max.x + 1).collect();
            if frame_counter.frame % 2 == 0 {
                y_range.reverse();
            }
            if frame_counter.frame / 2 % 2 == 0 {
                x_range.reverse();
            }

            for y in y_range.iter() {
                for x in x_range.iter() {
                    let local = Vector2I::new(*x, *y);
                    let global = local_to_global(&local, &chunk_index);

                    if terrain
                        .get_latest_simulation(&global)
                        .map_or(true, |frame| frame == simulation_frame)
                    {
                        continue;
                    };

                    simulate_texel(global, &mut terrain, &frame_counter);
                }
            }

            // Gas dispersion
            let alternate_dispersion = frame_counter.frame % 2 == 0;
            let alternate = if alternate_dispersion { 1 } else { 0 };
            let y_range =
                ((rect.min.y - alternate)..rect.max.y + 1 + alternate).collect::<Vec<_>>();
            let x_range =
                ((rect.min.x - alternate)..rect.max.x + 1 + alternate).collect::<Vec<_>>();
            const DISPERSION_WIDTH: usize = 2;
            const DISPERSION_HEIGHT: usize = 2;
            for y_arr in y_range.chunks(DISPERSION_HEIGHT) {
                for x_arr in x_range.chunks(DISPERSION_WIDTH) {
                    let mut global_positions = vec![];
                    for y in y_arr.iter() {
                        for x in x_arr.iter() {
                            let local = Vector2I::new(*x, *y);
                            let global = local_to_global(&local, &chunk_index);
                            global_positions.push(global);
                        }
                    }

                    // Distribute gas
                    disperse_gas(
                        global_positions,
                        &mut terrain,
                        &frame_counter,
                        &mut debug_draw,
                    )
                }
            }
        }
    }
}

// TODO: Don't update if the result of dispersion is similar to before
fn disperse_gas(
    global_positions: Vec<Vector2I>,
    terrain: &mut Terrain2D,
    frame_counter: &FrameCounter,
    debug_draw: &mut bevy_prototype_debug_lines::DebugLines,
) {
    use u32 as Capacity;
    use u8 as Min;
    use u8 as Max;
    let mut total_densities: HashMap<TexelID, (Capacity, Min, Max)> = HashMap::new();
    let mut valid_globals = vec![];
    for global in global_positions.iter() {
        let (texel, behaviour) = terrain.get_texel_behaviour(global);
        if behaviour.clone().map_or(true, |b| b.form == TexelForm::Gas) {
            valid_globals.push(*global);
        }
        match (texel, behaviour) {
            (Some(texel), Some(behaviour)) => {
                if behaviour.form == TexelForm::Gas {
                    if let Some((old_density, old_min, old_max)) = total_densities.get(&texel.id) {
                        total_densities.insert(
                            texel.id,
                            (
                                texel.density as u32 + *old_density,
                                texel.density.min(*old_min),
                                texel.density.max(*old_max),
                            ),
                        );
                    } else {
                        total_densities.insert(
                            texel.id,
                            (texel.density as u32, texel.density, texel.density),
                        );
                    }
                }
            }
            (_, _) => (),
        }
    }

    let mut total_densities: Vec<(TexelID, Capacity, Min, Max)> = total_densities
        .iter()
        .map(|(t, (d, min, max))| (*t, *d, *min, *max))
        .collect();

    if total_densities.len() == 0 {
        return;
    }

    total_densities.sort_unstable_by_key(|(_, density, _, _)| *density);
    total_densities.reverse();

    const TILE_CAPACITY: u32 = u8::MAX as u32;
    let free_slots = valid_globals.len() as u32
        - total_densities
            .iter()
            .map(|(_, v, _, _)| (*v / (TILE_CAPACITY + 1)) + 1)
            .sum::<u32>();

    // Stop if the gas is already close to a stable state
    const STABLE_TRESHOLD: u8 = 3;
    if total_densities.iter().all(|(_, _, min, max)| {
        if u8::abs_diff(*min, *max) > STABLE_TRESHOLD {
            return false;
        }
        free_slots > 0 && *max <= STABLE_TRESHOLD
    }) {
        // // DEBUG: draw box for stabilized area
        // let mut min = valid_globals.first().unwrap().clone();
        // let mut max = valid_globals.first().unwrap().clone();
        // for global in valid_globals.iter() {
        //     min = Vector2I::min(&min, global);
        //     max = Vector2I::max(&max, global);
        // }
        // max = max + Vector2I::ONE;
        // crate::game::debug::terrain::draw_box(
        //     debug_draw,
        //     Vec3::from(min),
        //     Vec3::from(max),
        //     Color::CYAN,
        //     0.0,
        // );
        return;
    }

    // Allocate slots
    let mut slots: Vec<(TexelID, u32)> = vec![];
    for (id, density, _, _) in total_densities.iter() {
        let min_slots = (density / (TILE_CAPACITY + 1)) + 1;
        slots.push((*id, min_slots));
    }
    for i in 0..free_slots as usize {
        let len = slots.len();
        slots[i % len].1 += 1;
    }

    // Disperse into given slots
    let mut texels: Vec<Texel2D> = vec![];
    for (id, total_density, _, _) in total_densities.iter() {
        let slots = slots.iter().find(|s| s.0 == *id).unwrap().1;
        let mut density_left = *total_density;
        for i in 0..slots {
            let density = if i < (slots - 1) {
                (total_density / slots).min(density_left)
            } else {
                density_left
            }
            .min(u8::MAX as u32);
            if density > 0 {
                texels.push(Texel2D {
                    id: *id,
                    density: density as u8,
                });
                density_left -= density;
            }
        }
    }

    // Apply changes
    if texels.len() > valid_globals.len() {
        panic!("disperse_gas() - valid_globals is shorter than texels");
    }

    fastrand::shuffle(&mut valid_globals);
    for i in 0..valid_globals.len() {
        let global = valid_globals[i];
        if i < texels.len() {
            let texel = texels[i];
            terrain.set_texel(&global, texel, None);
        } else {
            terrain.set_texel(&global, Texel2D::default(), None)
        }
    }
}

fn simulate_texel(global: Vector2I, terrain: &mut Terrain2D, frame_counter: &FrameCounter) {
    let (_, behaviour) = match terrain.get_texel_behaviour(&global) {
        (Some(texel), Some(behaviour)) => (texel, behaviour),
        (_, _) => return,
    };

    let simulation_frame = (frame_counter.frame % u8::MAX as u64) as u8 + 1;

    // Gravity
    if let Some(gravity) = behaviour.gravity {
        let grav_offset = Vector2I::from(gravity);
        let grav_pos = global + grav_offset;

        if behaviour.form != TexelForm::Gas || gravity.abs() > fastrand::u8(0..u8::MAX) {
            // Try falling
            {
                let (_, other_behaviour) = terrain.get_texel_behaviour(&grav_pos);
                if TexelBehaviour2D::can_displace(&behaviour, &other_behaviour) {
                    terrain.swap_texels(&global, &grav_pos, Some(simulation_frame));
                    return;
                }
                if terrain.can_transfer_density(&global, &grav_pos) {
                    terrain.transfer_density(&global, &grav_pos, gravity, Some(simulation_frame))
                }
            }

            // Try "sliding"
            let mut dirs = vec![Vector2I::RIGHT, Vector2I::LEFT];
            if ((frame_counter.frame / 73) % 2) as i32 == global.y % 2 {
                dirs.reverse();
            }
            for dir in dirs.iter() {
                let slide_pos = match behaviour.form {
                    TexelForm::Solid => grav_pos + *dir,
                    TexelForm::Liquid | TexelForm::Gas => global + *dir,
                };
                let (_, other_behaviour) = terrain.get_texel_behaviour(&slide_pos);
                if TexelBehaviour2D::can_displace(&behaviour, &other_behaviour) {
                    terrain.swap_texels(&global, &slide_pos, Some(simulation_frame));
                    return;
                }
                if terrain.can_transfer_density(&global, &grav_pos) {
                    terrain.transfer_density(&global, &grav_pos, gravity, Some(simulation_frame))
                }
            }
        }
    }
}

fn emit_terrain_events(
    mut terrain: ResMut<Terrain2D>,
    mut terrain_events: EventWriter<TerrainEvent2D>,
) {
    for event in terrain.events.drain(..) {
        terrain_events.send(event);
    }
    for (chunk_index, chunk) in terrain.chunk_iter_mut() {
        if let Some(rect) = &chunk.dirty_rect {
            terrain_events.send(TerrainEvent2D::TexelsUpdated(*chunk_index, *rect));
        }
    }
}

pub enum TerrainEvent2D {
    ChunkAdded(Chunk2DIndex),
    ChunkRemoved(Chunk2DIndex),
    TexelsUpdated(Chunk2DIndex, ChunkRect),
}

#[derive(Default, Resource)]
pub struct Terrain2D {
    chunk_map: HashMap<Chunk2DIndex, Chunk2D>,
    events: Vec<TerrainEvent2D>,
    pub top_boundary: Option<i32>,
    pub bottom_boundary: Option<i32>,
    pub left_boundary: Option<i32>,
    pub right_boundary: Option<i32>,
}

impl Terrain2D {
    pub const WORLD_WIDTH: i32 = 512;
    pub const WORLD_HEIGHT: i32 = Self::WORLD_WIDTH * 2;

    pub fn new(
        top_boundary: Option<i32>,
        bottom_boundary: Option<i32>,
        left_boundary: Option<i32>,
        right_boundary: Option<i32>,
    ) -> Self {
        Self {
            chunk_map: HashMap::new(),
            events: Vec::new(),
            top_boundary,
            bottom_boundary,
            left_boundary,
            right_boundary,
        }
    }

    pub fn add_chunk(&mut self, index: Chunk2DIndex, chunk: Chunk2D) {
        self.chunk_map.insert(index, chunk);
        self.events.push(TerrainEvent2D::ChunkAdded(index))
    }

    pub fn remove_chunk(&mut self, index: Chunk2DIndex) {
        self.events.push(TerrainEvent2D::ChunkRemoved(index));
        self.chunk_map.remove(&index);
    }

    pub fn chunk_iter(&self) -> Iter<Chunk2DIndex, Chunk2D> {
        self.chunk_map.iter()
    }

    pub fn chunk_iter_mut(&mut self) -> IterMut<Chunk2DIndex, Chunk2D> {
        self.chunk_map.iter_mut()
    }

    pub fn index_to_chunk(&self, index: &Chunk2DIndex) -> Option<&Chunk2D> {
        self.chunk_map.get(index)
    }

    pub fn index_to_chunk_mut(&mut self, index: &Chunk2DIndex) -> Option<&mut Chunk2D> {
        self.chunk_map.get_mut(index)
    }

    pub fn global_to_chunk(&self, global: &Vector2I) -> Option<&Chunk2D> {
        self.index_to_chunk(&global_to_chunk_index(global))
    }

    pub fn global_to_chunk_mut(&mut self, global: &Vector2I) -> Option<&mut Chunk2D> {
        self.index_to_chunk_mut(&global_to_chunk_index(global))
    }

    pub fn global_to_texel(&self, global: &Vector2I) -> Option<Texel2D> {
        match self.global_to_chunk(global) {
            Some(chunk) => chunk.get_texel(&global_to_local(global)),
            None => None,
        }
    }

    pub fn global_to_texel_mut(&mut self, global: &Vector2I) -> Option<Texel2D> {
        match self.global_to_chunk(global) {
            Some(chunk) => chunk.get_texel(&global_to_local(global)),
            None => None,
        }
    }

    pub fn mark_dirty(&mut self, global: &Vector2I) {
        let index = global_to_chunk_index(global);
        if let Some(chunk) = self.index_to_chunk_mut(&index) {
            chunk.mark_dirty(&global_to_local(global));
        }
    }

    pub fn is_within_boundaries(&self, global: &Vector2I) -> bool {
        if let Some(top) = self.top_boundary {
            if global.y >= top {
                return false;
            }
        }
        if let Some(bottom) = self.bottom_boundary {
            if global.y < bottom {
                return false;
            }
        }
        if let Some(left) = self.left_boundary {
            if global.x < left {
                return false;
            }
        }
        if let Some(right) = self.right_boundary {
            if global.x >= right {
                return false;
            }
        }
        return true;
    }

    pub fn get_texel(&self, global: &Vector2I) -> Option<Texel2D> {
        self.global_to_chunk(global)
            .map_or(None, |chunk| chunk.get_texel(&global_to_local(global)))
    }

    pub fn get_latest_simulation(&self, global: &Vector2I) -> Option<u8> {
        self.global_to_chunk(global).map_or(None, |chunk| {
            chunk.get_latest_simulation(&global_to_local(global))
        })
    }

    pub fn get_texel_behaviour(
        &self,
        global: &Vector2I,
    ) -> (Option<Texel2D>, Option<TexelBehaviour2D>) {
        let texel = self.get_texel(global);
        (
            texel,
            if self.is_within_boundaries(global) {
                texel.map_or(None, |t| TexelBehaviour2D::from_id(&t.id))
            } else {
                Some(TexelBehaviour2D::OUT_OF_BOUNDS)
            },
        )
    }

    pub fn set_texel(
        &mut self,
        global: &Vector2I,
        new_texel: Texel2D,
        simulation_frame: Option<u8>,
    ) {
        if !self.is_within_boundaries(global) {
            return;
        }
        let index = global_to_chunk_index(global);
        let changed = match self.index_to_chunk_mut(&index) {
            Some(chunk) => chunk.set_texel(&global_to_local(global), new_texel, simulation_frame),
            None => {
                let mut chunk = Chunk2D::new();
                let changed =
                    chunk.set_texel(&global_to_local(global), new_texel, simulation_frame);
                self.add_chunk(index, chunk);
                changed
            }
        };
        if changed {
            self.mark_dirty(&(*global + Vector2I::UP));
            self.mark_dirty(&(*global + Vector2I::RIGHT));
            self.mark_dirty(&(*global + Vector2I::DOWN));
            self.mark_dirty(&(*global + Vector2I::LEFT));
        }
    }

    pub fn swap_texels(
        &mut self,
        from_global: &Vector2I,
        to_global: &Vector2I,
        simulation_frame: Option<u8>,
    ) {
        let from = self.get_texel(from_global).unwrap_or_default();
        let to = self.get_texel(to_global).unwrap_or_default();
        self.set_texel(to_global, from, simulation_frame);
        // REM: The displaced texel is also marked as simulated
        self.set_texel(from_global, to, simulation_frame);
    }

    fn can_transfer_density(&self, from_global: &Vector2I, to_global: &Vector2I) -> bool {
        let from = self.get_texel(from_global).unwrap_or_default();
        let to = self.get_texel(to_global).unwrap_or_default();
        if from.id != to.id {
            return false;
        }

        let behaviour = if let Some(behaviour) = from.behaviour() {
            behaviour
        } else {
            return false;
        };

        behaviour.form == TexelForm::Gas
    }

    fn transfer_density(
        &mut self,
        from_global: &Vector2I,
        to_global: &Vector2I,
        gravity: TexelGravity,
        simulation_frame: Option<u8>,
    ) {
        let from = self.get_texel(from_global).unwrap_or_default();
        let to = self.get_texel(to_global).unwrap_or_default();
        let max_transfer = gravity.abs();

        // DEBUG: Test this out, another property?
        const MAX_TARGET_DENSITY: u8 = 25;
        let transfer = (u8::MAX - to.density)
            .min(max_transfer)
            .min(from.density)
            .min(MAX_TARGET_DENSITY.max(to.density) - to.density);
        if transfer == 0 {
            return;
        }

        if from.density - transfer == 0 {
            self.set_texel(&from_global, Texel2D::default(), simulation_frame);
        } else {
            self.set_texel(
                &from_global,
                Texel2D {
                    density: from.density - transfer,
                    ..from
                },
                simulation_frame,
            );
        }
        self.set_texel(
            &to_global,
            Texel2D {
                density: to.density + transfer,
                ..to
            },
            simulation_frame,
        );
    }
}

pub fn local_to_texel_index(position: &Vector2I) -> Option<usize> {
    match position.x >= 0
        && position.y >= 0
        && position.x < Chunk2D::SIZE.x
        && position.y < Chunk2D::SIZE.y
    {
        true => Some(position.y as usize * Chunk2D::SIZE_X + position.x as usize),
        false => None,
    }
}

pub fn texel_index_to_local(i: usize) -> Vector2I {
    Vector2I {
        x: i as i32 % Chunk2D::SIZE.x,
        y: i as i32 / Chunk2D::SIZE.y,
    }
}

pub fn texel_index_to_global(i: usize, chunk_index: &Chunk2DIndex) -> Vector2I {
    Vector2I {
        x: i as i32 % Chunk2D::SIZE.x,
        y: i as i32 / Chunk2D::SIZE.y,
    } + chunk_index_to_global(chunk_index)
}

pub fn local_to_global(local: &Vector2I, chunk_index: &Chunk2DIndex) -> Vector2I {
    chunk_index_to_global(chunk_index) + *local
}

pub fn global_to_local(global: &Vector2I) -> Vector2I {
    Vector2I {
        x: wrapping_remainder(global.x, Chunk2D::SIZE.x),
        y: wrapping_remainder(global.y, Chunk2D::SIZE.y),
    }
}

pub fn global_to_chunk_index(position: &Vector2I) -> Chunk2DIndex {
    Vector2I {
        x: wrapping_quotient(position.x, Chunk2D::SIZE.x),
        y: wrapping_quotient(position.y, Chunk2D::SIZE.y),
    }
}

pub fn chunk_index_to_global(chunk_pos: &Chunk2DIndex) -> Vector2I {
    *chunk_pos * Chunk2D::SIZE
}
