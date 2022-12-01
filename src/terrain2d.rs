use std::collections::{
    hash_map::{Iter, IterMut},
    HashMap,
};

use bevy::prelude::*;

mod chunk2d;
mod texel2d;

pub use chunk2d::*;
pub use texel2d::*;

use crate::util::{math::*, Vector2I};

pub struct Terrain2DPlugin;

impl Plugin for Terrain2DPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Terrain2D::new())
            .add_system(emit_terrain_events);
    }
}

fn emit_terrain_events(
    mut terrain: ResMut<Terrain2D>,
    mut terrain_events: EventWriter<TerrainEvent>,
) {
    for event in terrain.events.drain(..) {
        terrain_events.send(event)
    }
    for (chunk_index, mut chunk) in terrain.chunk_iter_mut() {
        if let Some(rect) = &chunk.dirty_rect {
            terrain_events.send(TerrainEvent::TexelsUpdated(*chunk_index, *rect));
            chunk.dirty_rect = None;
        }
    }
}

pub enum TerrainEvent {
    ChunkAdded(ChunkIndex),
    ChunkRemoved(ChunkIndex),
    TexelsUpdated(ChunkIndex, ChunkRect),
}

#[derive(Default, Resource)]
pub struct Terrain2D {
    chunk_map: HashMap<ChunkIndex, Chunk>,
    events: Vec<TerrainEvent>,
}

impl Terrain2D {
    pub fn new() -> Terrain2D {
        Terrain2D {
            chunk_map: HashMap::new(),
            events: Vec::new(),
        }
    }

    pub fn add_chunk(&mut self, index: ChunkIndex, chunk: Chunk) {
        self.chunk_map.insert(index, chunk);
        self.events.push(TerrainEvent::ChunkAdded(index))
    }

    pub fn remove_chunk(&mut self, index: ChunkIndex) {
        self.events.push(TerrainEvent::ChunkRemoved(index));
        self.chunk_map.remove(&index);
    }

    pub fn chunk_iter(&self) -> Iter<ChunkIndex, Chunk> {
        self.chunk_map.iter()
    }

    pub fn chunk_iter_mut(&mut self) -> IterMut<ChunkIndex, Chunk> {
        self.chunk_map.iter_mut()
    }

    pub fn index_to_chunk(&self, index: &ChunkIndex) -> Option<&Chunk> {
        self.chunk_map.get(index)
    }

    pub fn index_to_chunk_mut(&mut self, index: &ChunkIndex) -> Option<&mut Chunk> {
        self.chunk_map.get_mut(index)
    }

    pub fn global_to_chunk(&self, global: &Vector2I) -> Option<&Chunk> {
        self.index_to_chunk(&global_to_chunk_index(global))
    }

    pub fn global_to_chunk_mut(&mut self, global: &Vector2I) -> Option<&mut Chunk> {
        self.index_to_chunk_mut(&global_to_chunk_index(global))
    }

    pub fn global_to_texel(&self, global: &Vector2I) -> Option<Texel> {
        match self.global_to_chunk(global) {
            Some(chunk) => chunk.get_texel(&global_to_local(global)),
            None => None,
        }
    }

    pub fn global_to_texel_mut(&mut self, global: &Vector2I) -> Option<Texel> {
        match self.global_to_chunk(global) {
            Some(chunk) => chunk.get_texel(&global_to_local(global)),
            None => None,
        }
    }

    pub fn set_texel(&mut self, global: &Vector2I, id: TexelID) {
        let index = global_to_chunk_index(global);
        match self.index_to_chunk_mut(&index) {
            Some(chunk) => chunk.set_texel(&global_to_local(global), id),
            None => {
                let mut chunk = Chunk::new();
                chunk.set_texel(&global_to_local(global), id);
                self.add_chunk(index, chunk);
            }
        }
    }
}

pub fn local_to_texel_index(position: &Vector2I) -> Option<usize> {
    match position.x >= 0
        && position.y >= 0
        && position.x < Chunk::SIZE.x
        && position.y < Chunk::SIZE.y
    {
        true => Some(position.y as usize * Chunk::SIZE_X + position.x as usize),
        false => None,
    }
}

pub fn texel_index_to_local(i: usize) -> Vector2I {
    Vector2I {
        x: i as i32 % Chunk::SIZE.x,
        y: i as i32 / Chunk::SIZE.y,
    }
}

pub fn global_to_local(position: &Vector2I) -> Vector2I {
    Vector2I {
        x: wrapping_remainder(position.x, Chunk::SIZE.x),
        y: wrapping_remainder(position.y, Chunk::SIZE.y),
    }
}

pub fn global_to_chunk_index(position: &Vector2I) -> ChunkIndex {
    Vector2I {
        x: wrapping_quotient(position.x, Chunk::SIZE.x),
        y: wrapping_quotient(position.y, Chunk::SIZE.y),
    }
}

pub fn chunk_index_to_global(chunk_pos: &ChunkIndex) -> Vector2I {
    *chunk_pos * Chunk::SIZE
}
