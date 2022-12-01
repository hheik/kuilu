use super::{local_to_texel_index, Texel, TexelID, NEIGHBOUR_INDEX_MAP};
use crate::util::Vector2I;

pub type ChunkIndex = Vector2I;

#[derive(Clone, Copy)]
pub struct ChunkRect {
    pub min: Vector2I,
    pub max: Vector2I,
}

pub struct Chunk {
    pub texels: [Texel; (Self::SIZE_X * Self::SIZE_Y) as usize],
    // TODO: handle multiple dirty rects
    pub dirty_rect: Option<ChunkRect>,
}

impl Chunk {
    pub const SIZE_X: usize = 64;
    pub const SIZE_Y: usize = 64;
    pub const SIZE: Vector2I = Vector2I {
        x: Self::SIZE_X as i32,
        y: Self::SIZE_Y as i32,
    };

    pub fn new() -> Chunk {
        Chunk {
            texels: Self::new_texel_array(),
            dirty_rect: None,
        }
    }

    pub fn new_texel_array() -> [Texel; Self::SIZE_X * Self::SIZE_Y] {
        [Texel::default(); Self::SIZE_X * Self::SIZE_Y]
    }

    pub fn mark_all_dirty(&mut self) {
        self.dirty_rect = Some(ChunkRect {
            min: Vector2I::ZERO,
            max: Self::SIZE,
        });
    }

    pub fn mark_dirty(&mut self, position: &Vector2I) {
        match &self.dirty_rect {
            Some(rect) => {
                self.dirty_rect = Some(ChunkRect {
                    min: Vector2I::min(&rect.min, position),
                    max: Vector2I::max(&rect.max, position),
                })
            }
            None => {
                self.dirty_rect = Some(ChunkRect {
                    min: *position,
                    max: *position,
                })
            }
        }
    }

    pub fn get_texel(&self, position: &Vector2I) -> Option<Texel> {
        local_to_texel_index(position).map(|i| self.texels[i])
    }

    pub fn get_texel_option_mut(&mut self, position: &Vector2I) -> Option<&mut Texel> {
        local_to_texel_index(position).map(|i| &mut self.texels[i])
    }

    pub fn set_texel(&mut self, position: &Vector2I, id: TexelID) {
        let i = local_to_texel_index(position).expect("Texel index out of range");
        if self.texels[i].id != id {
            self.mark_dirty(position);
        }
        let update_neighbours = self.texels[i].is_empty()
            != (Texel {
                id,
                ..self.texels[i]
            })
            .is_empty();
        self.texels[i].id = id;
        // Update neighbour mask
        if update_neighbours {
            for offset in Texel::NEIGHBOUR_OFFSET_VECTORS {
                // Flip neighbour's bit
                match self.get_texel_option_mut(&(*position + offset)) {
                    Some(mut neighbour) => {
                        neighbour.neighbour_mask ^= 1 << NEIGHBOUR_INDEX_MAP[&-offset];
                    }
                    None => (),
                }
            }
        }
    }
}
