use noise::{NoiseFn, PerlinSurflet};

use super::*;
use crate::util::{inverse_lerp, lerp};

pub struct TerrainGen2D {
    pub seed: u32,
    noise: PerlinSurflet,
}

impl TerrainGen2D {
    const NOISE_SCALE: f64 = 1.0;

    pub fn new(seed: u32) -> TerrainGen2D {
        let noise = PerlinSurflet::new(seed);
        TerrainGen2D { noise, seed }
    }

    pub fn gen_chunk(&self, position: &Chunk2DIndex) -> Chunk2D {
        let mut chunk = Chunk2D::new();
        for local in Chunk2D::xy_vec().iter() {
            let global = chunk_index_to_global(position) + *local;

            let x = global.x as f64 * Self::NOISE_SCALE;
            let y = global.y as f64 * Self::NOISE_SCALE;

            let mut value = 0.5;
            value += self.noise.get([x / 115.0, y / 1.25 / 115.0]);
            value += self.noise.get([x / 77.0, y / 77.0]) * 0.3;
            value += self.noise.get([x / 17.0, y / 17.0]) * 0.05;

            let mut id = 0;
            if value > 0.35 {
                id = 11;
            }
            if value > 0.42 {
                id = 12;
            }
            if value > 0.9 {
                id = 13;
            }

            chunk.set_texel(&local, Texel2D { id, ..default() }, None);
        }
        chunk
    }
}
