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
            .insert_resource(Terrain2D::new())
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

// TODO: Add simulation boundaries
fn terrain_simulation(mut terrain: ResMut<Terrain2D>, frame_counter: Res<FrameCounter>) {
    let simulation_frame = (frame_counter.frame % u8::MAX as u64) as u8 + 1;

    let indices = terrain
        .chunk_iter()
        .map(|(chunk_index, _)| *chunk_index)
        .collect::<Vec<Chunk2DIndex>>()
        .clone();

    for chunk_index in indices.iter() {
        // // DEBUG: mark few chunks dirty in interval
        // if let Some(chunk) = terrain.index_to_chunk_mut(&chunk_index) {
        //     let interval = 2;
        //     if frame_counter.frame % interval == 0 {
        //         let i = ((frame_counter.frame / interval) % 100) as i32;
        //         if (chunk_index.y % 10) * 10 + (chunk_index.x % 10) == i {
        //             chunk.mark_all_dirty();
        //             println!("chunk {:?} is now dirty", chunk_index);
        //         }
        //     }
        // };

        if let Some(rect) = &terrain
            .index_to_chunk(&chunk_index)
            .map_or(None, |chunk| chunk.dirty_rect.clone())
        {
            if let Some(chunk) = terrain.index_to_chunk_mut(&chunk_index) {
                chunk.mark_clean();
            };
            let mut y_range: Vec<_> = (rect.min.y..rect.max.y + 1).collect();
            let mut x_range: Vec<_> = (rect.min.x..rect.max.x + 1).collect();

            if frame_counter.frame % 2 == 0 {
                y_range.reverse();
            }
            if frame_counter.frame / 2 % 2 == 0 {
                x_range.reverse();
            }

            for y in y_range.iter() {
                'texel_loop: for x in x_range.iter() {
                    let local = Vector2I::new(*x, *y);
                    let global = local_to_global(&local, &chunk_index);

                    let texel = if let Some(texel) = terrain.get_texel(&global) {
                        if texel.last_simulation == simulation_frame {
                            continue 'texel_loop;
                        }
                        texel
                    } else {
                        continue;
                    };
                    let tb = if let Some(tb) = TexelBehaviour2D::from_id(&texel.id) {
                        tb
                    } else {
                        continue;
                    };

                    match tb.form {
                        TexelForm::Liquid => {
                            // Check if there is space below
                            {
                                let below_pos = global + Vector2I::DOWN;
                                if terrain.get_texel(&below_pos).map_or(true, |texel| {
                                    TexelBehaviour2D::is_empty(&texel.id)
                                        || TexelBehaviour2D::is_gas(&texel.id)
                                }) {
                                    let below_id =
                                        terrain.get_texel(&below_pos).map_or(0, |texel| texel.id);
                                    terrain.set_texel(&below_pos, texel.id, Some(simulation_frame));
                                    terrain.set_texel(&global, below_id, Some(simulation_frame));
                                    continue;
                                }
                            }

                            // Check if there is space to the side
                            let mut dirs = vec![Vector2I::RIGHT, Vector2I::LEFT];
                            if (frame_counter.frame / 3) % 2 == 0 {
                                dirs.reverse();
                            }
                            for dir in dirs.iter() {
                                let side_pos = global + *dir;
                                if terrain.get_texel(&side_pos).map_or(true, |texel| {
                                    TexelBehaviour2D::is_empty(&texel.id)
                                        || TexelBehaviour2D::is_gas(&texel.id)
                                }) {
                                    let side_id =
                                        terrain.get_texel(&side_pos).map_or(0, |texel| texel.id);
                                    terrain.set_texel(&side_pos, texel.id, Some(simulation_frame));
                                    terrain.set_texel(&global, side_id, Some(simulation_frame));
                                    continue 'texel_loop;
                                };
                            }
                        }
                        _ => (),
                    }
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
}

impl Terrain2D {
    pub fn new() -> Terrain2D {
        Terrain2D {
            chunk_map: HashMap::new(),
            events: Vec::new(),
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

    pub fn get_texel(&self, global: &Vector2I) -> Option<Texel2D> {
        self.global_to_chunk(global)
            .map_or(None, |chunk| chunk.get_texel(&global_to_local(global)))
    }

    pub fn set_texel(&mut self, global: &Vector2I, id: TexelID, simulation_frame: Option<u8>) {
        let index = global_to_chunk_index(global);
        let changed = match self.index_to_chunk_mut(&index) {
            Some(chunk) => chunk.set_texel(&global_to_local(global), id, simulation_frame),
            None => {
                let mut chunk = Chunk2D::new();
                let changed = chunk.set_texel(&global_to_local(global), id, simulation_frame);
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
