use std::collections::HashMap;

use super::{local_to_texel_index, Terrain2D, TerrainEvent, Texel2D, TexelID, NEIGHBOUR_INDEX_MAP};
use crate::util::Vector2I;
use bevy::{prelude::*, render::{render_resource::Extent3d, texture::ImageSampler}};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref COLOR_MAP: HashMap<TexelID, [u8; 4]> = {
        let mut map = HashMap::new();
        map.insert(0, [0x20, 0x20, 0x20, 0xff]);
        map.insert(1, [0xff, 0xff, 0xff, 0xff]);
        map
    };
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Chunk2DIndex {
    pub index: ChunkIndex,
}

#[derive(Bundle, Default)]
pub struct ChunkBundle {
    pub chunk: Chunk2DIndex,
    pub sprite_bundle: SpriteBundle,
}

pub type ChunkIndex = Vector2I;

#[derive(Clone, Copy)]
pub struct ChunkRect {
    pub min: Vector2I,
    pub max: Vector2I,
}

pub struct Chunk2D {
    pub texels: [Texel2D; (Self::SIZE_X * Self::SIZE_Y) as usize],
    // TODO: handle multiple dirty rects
    pub dirty_rect: Option<ChunkRect>,
}

impl Chunk2D {
    pub const SIZE_X: usize = 64;
    pub const SIZE_Y: usize = 64;
    pub const SIZE: Vector2I = Vector2I {
        x: Self::SIZE_X as i32,
        y: Self::SIZE_Y as i32,
    };

    pub fn new() -> Chunk2D {
        Chunk2D {
            texels: Self::new_texel_array(),
            dirty_rect: None,
        }
    }

    pub fn new_full() -> Chunk2D {
        let mut chunk = Chunk2D {
            texels: Self::new_texel_array(),
            dirty_rect: None,
        };
        for y in 0..Self::SIZE_Y {
            for x in 0..Self::SIZE_X {
                chunk.set_texel(&Vector2I::new(x as i32, y as i32), 1);
            }
        }
        chunk
    }

    pub fn new_half() -> Chunk2D {
        let mut chunk = Chunk2D {
            texels: Self::new_texel_array(),
            dirty_rect: None,
        };
        for y in 0..Self::SIZE_Y {
            for x in 0..Self::SIZE_X {
                if x <= Self::SIZE_Y - y {
                    chunk.set_texel(&Vector2I::new(x as i32, y as i32), 1);
                }
            }
        }
        chunk
    }

    pub fn new_circle() -> Chunk2D {
        let mut chunk = Chunk2D {
            texels: Self::new_texel_array(),
            dirty_rect: None,
        };
        let origin = Self::SIZE / 2;
        let radius = Self::SIZE_X as i32 / 2;
        for y in 0..Self::SIZE_Y {
            for x in 0..Self::SIZE_X {
                let dx = (x as i32 - origin.x).abs();
                let dy = (y as i32 - origin.y).abs();
                if dx * dx + dy * dy <= (radius - 1) * (radius - 1) {
                    chunk.set_texel(&Vector2I::new(x as i32, y as i32), 1);
                }
            }
        }
        chunk
    }

    pub fn new_texel_array() -> [Texel2D; Self::SIZE_X * Self::SIZE_Y] {
        [Texel2D::default(); Self::SIZE_X * Self::SIZE_Y]
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

    pub fn get_texel(&self, position: &Vector2I) -> Option<Texel2D> {
        local_to_texel_index(position).map(|i| self.texels[i])
    }

    pub fn get_texel_mut(&mut self, position: &Vector2I) -> Option<&mut Texel2D> {
        local_to_texel_index(position).map(|i| &mut self.texels[i])
    }

    pub fn set_texel(&mut self, position: &Vector2I, id: TexelID) {
        let i = local_to_texel_index(position).expect("Texel index out of range");
        if self.texels[i].id != id {
            self.mark_dirty(position);
        }
        let update_neighbours = self.texels[i].is_empty()
            != (Texel2D {
                id,
                ..self.texels[i]
            })
            .is_empty();
        self.texels[i].id = id;
        // Update neighbour mask
        if update_neighbours {
            for offset in Texel2D::NEIGHBOUR_OFFSET_VECTORS {
                // Flip neighbour's bit
                match self.get_texel_mut(&(*position + offset)) {
                    Some(mut neighbour) => {
                        neighbour.neighbour_mask ^= 1 << NEIGHBOUR_INDEX_MAP[&-offset];
                    }
                    None => (),
                }
            }
        }
    }
}

pub fn chunk_spawner(
    mut commands: Commands,
    mut terrain_events: EventReader<TerrainEvent>,
    mut images: ResMut<Assets<Image>>,
    terrain: Res<Terrain2D>,
    chunk_query: Query<(Entity, &Chunk2DIndex)>,
) {
    for terrain_event in terrain_events.iter() {
        match terrain_event {
            TerrainEvent::ChunkAdded(chunk_index) => {
                let chunk = terrain.index_to_chunk(chunk_index).unwrap();

                let mut data = Vec::with_capacity(Chunk2D::SIZE_X * Chunk2D::SIZE_Y * 4);
                let fallback: [u8; 4] = [0x00, 0x00, 0x00, 0x00];
                for y in (0..Chunk2D::SIZE_Y).rev() {
                    for x in 0..Chunk2D::SIZE_X {
                        data.append(
                            &mut COLOR_MAP
                                .get(
                                    &chunk
                                        .get_texel(&Vector2I::new(x as i32, y as i32))
                                        .unwrap()
                                        .id,
                                )
                                .unwrap_or(&fallback)
                                .to_vec()
                                .clone(),
                        );
                    }
                }
                let mut image = Image::new(
                    Extent3d {
                        width: Chunk2D::SIZE_X as u32,
                        height: Chunk2D::SIZE_Y as u32,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    data,
                    bevy::render::render_resource::TextureFormat::Rgba8Unorm,
                );

                image.sampler_descriptor = ImageSampler::nearest();

                let texture = images.add(image);

                let pos = Vec2::from(*chunk_index * Chunk2D::SIZE);
                commands
                    .spawn(ChunkBundle {
                        chunk: Chunk2DIndex {
                            index: *chunk_index,
                        },
                        sprite_bundle: SpriteBundle {
                            sprite: Sprite {
                                color: Color::rgb(
                                    0.25 + (chunk_index.x % 4) as f32 * 0.25,
                                    0.25 + (chunk_index.y % 4) as f32 * 0.25,
                                    0.75,
                                ),
                                custom_size: Some(Vec2::from(Chunk2D::SIZE)),
                                anchor: bevy::sprite::Anchor::BottomLeft,
                                ..default()
                            },
                            texture,
                            transform: Transform::from_translation(Vec3::new(pos.x, pos.y, 0.0)),
                            ..default()
                        },
                    })
                    .insert(Name::new(format!(
                        "Chunk {},{}",
                        chunk_index.x, chunk_index.y
                    )));
            }
            TerrainEvent::ChunkRemoved(chunk_index) => {
                for (entity, chunk) in chunk_query.iter() {
                    if chunk.index == *chunk_index {
                        commands.entity(entity).despawn_recursive();
                    }
                }
            }
            TerrainEvent::TexelsUpdated(chunk_index, rect) => {}
        }
    }
}
