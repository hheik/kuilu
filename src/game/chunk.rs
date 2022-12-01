use bevy::prelude::*;

use crate::terrain2d::ChunkIndex;

pub struct ChunkPlugin {}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Chunk {
    pub index: ChunkIndex,
}

#[derive(Bundle)]
pub struct ChunkBundle {
    pub chunk: Chunk,
    pub sprite_bundle: SpriteBundle,
}
