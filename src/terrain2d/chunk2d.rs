use super::{local_to_texel_index, Terrain2D, TerrainEvent, Texel2D, TexelID, NEIGHBOUR_INDEX_MAP};
use crate::util::Vector2I;
use bevy::{prelude::*, render::render_resource::Extent3d};

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

    pub fn get_texel_option_mut(&mut self, position: &Vector2I) -> Option<&mut Texel2D> {
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
                let mut data = Vec::with_capacity(Chunk2D::SIZE_X * Chunk2D::SIZE_Y * 4);
                for _y in 0..Chunk2D::SIZE_Y {
                    for _x in 0..Chunk2D::SIZE_X {
                        data.push(0x00);
                        data.push(0x00);
                        data.push(0x00);
                        data.push(0x00);
                    }
                }
                let image = Image::new(
                    Extent3d {
                        width: Chunk2D::SIZE_X as u32,
                        height: Chunk2D::SIZE_Y as u32,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    data,
                    bevy::render::render_resource::TextureFormat::Rgba8Unorm,
                );

                images.add(image);

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
                                ..default()
                            },
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
