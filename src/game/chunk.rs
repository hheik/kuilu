use bevy::prelude::*;

use crate::terrain2d::Chunk2DIndex;

pub struct ChunkPlugin {}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Chunk {
    pub index: Chunk2DIndex,
}

#[derive(Bundle)]
pub struct ChunkBundle {
    pub chunk: Chunk,
    pub sprite_bundle: SpriteBundle,
}
