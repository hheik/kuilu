use std::collections::VecDeque;

use super::{
    local_to_texel_index, texel_index_to_local, Terrain2D, TerrainEvent2D, Texel2D,
    TexelBehaviour2D, TexelID, NEIGHBOUR_INDEX_MAP,
};
use crate::util::{CollisionLayers, Segment2I, Vector2I};
use bevy::{
    prelude::*,
    render::{render_resource::Extent3d, texture::ImageSampler},
};
use bevy_rapier2d::prelude::*;
use lazy_static::lazy_static;

type Island = VecDeque<Segment2I>;

lazy_static! {
    /// Marching Square case dictionary.
    ///
    /// Key is a bitmask of neighbouring tiles (up, right, down, left - least significant bit first).
    /// Bit set to 1 means that the neighbour has collision. Only the 4 least significant bits are currently used.
    ///
    /// Value is an array of segments that the tile should have. The segments are configured to go clockwise.
    ///
    /// Note: This dictionary should only be used for empty tiles.
    static ref MST_CASE_MAP: [Vec<Segment2I>; 16] = [
        /* 0b0000 */ vec![],
        /* 0b0001 */ vec![ Segment2I { from: Vector2I::ONE, to: Vector2I::UP } ],
        /* 0b0010 */ vec![ Segment2I { from: Vector2I::RIGHT, to: Vector2I::ONE } ],
        /* 0b0011 */ vec![ Segment2I { from: Vector2I::RIGHT, to: Vector2I::UP } ],
        /* 0b0100 */ vec![ Segment2I { from: Vector2I::ZERO, to: Vector2I::RIGHT } ],
        /* 0b0101 */ vec![ Segment2I { from: Vector2I::ONE, to: Vector2I::UP }, Segment2I { from: Vector2I::ZERO, to: Vector2I::RIGHT } ],
        /* 0b0110 */ vec![ Segment2I { from: Vector2I::ZERO, to: Vector2I::ONE } ],
        /* 0b0111 */ vec![ Segment2I { from: Vector2I::ZERO, to: Vector2I::UP } ],
        /* 0b1000 */ vec![ Segment2I { from: Vector2I::UP, to: Vector2I::ZERO } ],
        /* 0b1001 */ vec![ Segment2I { from: Vector2I::ONE, to: Vector2I::ZERO } ],
        /* 0b1010 */ vec![ Segment2I { from: Vector2I::RIGHT, to: Vector2I::ONE }, Segment2I { from: Vector2I::UP, to: Vector2I::ZERO } ],
        /* 0b1011 */ vec![ Segment2I { from: Vector2I::RIGHT, to: Vector2I::ZERO } ],
        /* 0b1100 */ vec![ Segment2I { from: Vector2I::UP, to: Vector2I::RIGHT } ],
        /* 0b1101 */ vec![ Segment2I { from: Vector2I::ONE, to: Vector2I::RIGHT } ],
        /* 0b1110 */ vec![ Segment2I { from: Vector2I::UP, to: Vector2I::ONE } ],
        /* 0b1111 */ vec![],
    ];

    /// Version of the MS case dictionary that is used by the solid tiles at the edge of the chunk
    static ref MST_EDGE_CASE_MAP: [Segment2I; 4] = [
        /* up    */ Segment2I { from: Vector2I::UP, to: Vector2I::ONE },
        /* right */ Segment2I { from: Vector2I::ONE, to: Vector2I::RIGHT },
        /* down  */ Segment2I { from: Vector2I::RIGHT, to: Vector2I::ZERO },
        /* left  */ Segment2I { from: Vector2I::ZERO, to: Vector2I::UP },
    ];
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct TerrainChunk2D {
    pub index: Chunk2DIndex,
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct TerrainChunkSpriteSync2D;

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct TerrainChunkCollisionSync2D;

#[derive(Bundle, Default)]
pub struct ChunkSpriteBundle {
    pub chunk: TerrainChunk2D,
    pub sync_flag: TerrainChunkSpriteSync2D,
    pub sprite: SpriteBundle,
}

#[derive(Bundle, Default)]
pub struct ChunkColliderBundle {
    pub chunk: TerrainChunk2D,
    pub sync_flag: TerrainChunkCollisionSync2D,
    pub transform: TransformBundle,
}

pub type Chunk2DIndex = Vector2I;

#[derive(Clone, Copy)]
pub struct ChunkRect {
    pub min: Vector2I,
    pub max: Vector2I,
}

impl ChunkRect {
    pub fn include_point(&self, point: Vector2I) -> Self {
        ChunkRect {
            min: Vector2I::min(&self.min, &point),
            max: Vector2I::max(&self.max, &point),
        }
    }
}

pub struct Chunk2D {
    pub texels: [Texel2D; (Self::SIZE_X * Self::SIZE_Y) as usize],
    // TODO: handle multiple dirty rects?
    pub dirty_rect: Option<ChunkRect>,
}

impl Chunk2D {
    pub const SIZE_X: usize = 32;
    pub const SIZE_Y: usize = 32;
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
                chunk.set_texel(&Vector2I::new(x as i32, y as i32), 1, None);
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
                    chunk.set_texel(&Vector2I::new(x as i32, y as i32), 1, None);
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
                    chunk.set_texel(&Vector2I::new(x as i32, y as i32), 1, None);
                }
            }
        }
        chunk
    }

    pub fn new_texel_array() -> [Texel2D; Self::SIZE_X * Self::SIZE_Y] {
        [Texel2D::default(); Self::SIZE_X * Self::SIZE_Y]
    }

    pub fn xy_vec() -> Vec<Vector2I> {
        let mut result = Vec::with_capacity(Self::SIZE_X * Self::SIZE_Y);
        for y in 0..Self::SIZE_Y {
            for x in 0..Self::SIZE_X {
                result.push(Vector2I {
                    x: x as i32,
                    y: y as i32,
                });
            }
        }
        result
    }

    pub fn mark_all_dirty(&mut self) {
        self.dirty_rect = Some(ChunkRect {
            min: Vector2I::ZERO,
            max: Self::SIZE - Vector2I::ONE,
        });
    }

    pub fn mark_dirty(&mut self, position: &Vector2I) {
        match &self.dirty_rect {
            Some(rect) => self.dirty_rect = Some(rect.include_point(*position)),
            None => {
                self.dirty_rect = Some(ChunkRect {
                    min: *position,
                    max: *position,
                })
            }
        }
    }

    pub fn mark_clean(&mut self) {
        self.dirty_rect = None;
    }

    pub fn get_texel(&self, position: &Vector2I) -> Option<Texel2D> {
        local_to_texel_index(position).map(|i| self.texels[i])
    }

    pub fn get_texel_mut(&mut self, position: &Vector2I) -> Option<&mut Texel2D> {
        local_to_texel_index(position).map(|i| &mut self.texels[i])
    }

    pub fn set_texel(
        &mut self,
        position: &Vector2I,
        id: TexelID,
        simulation_frame: Option<u8>,
    ) -> bool {
        let i = local_to_texel_index(position).expect("Texel index out of range");
        if self.texels[i].id != id {
            self.mark_dirty(position);
        }
        let update_neighbours = TexelBehaviour2D::has_collision(&self.texels[i].id)
            != TexelBehaviour2D::has_collision(&id);
        let changed = self.texels[i].id != id;
        self.texels[i].id = id;
        if let Some(simulation_frame) = simulation_frame {
            self.texels[i].last_simulation = simulation_frame;
        }
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
        changed
    }

    pub fn create_texture_data(&self) -> Vec<u8> {
        let mut image_data = Vec::with_capacity(Chunk2D::SIZE_X * Chunk2D::SIZE_Y * 4);
        for y in (0..Chunk2D::SIZE_Y).rev() {
            for x in 0..Chunk2D::SIZE_X {
                let id = &self
                    .get_texel(&Vector2I::new(x as i32, y as i32))
                    .unwrap()
                    .id;
                let behaviour = TexelBehaviour2D::from_id(id);
                let color =
                    behaviour.map_or(Color::rgba_u8(0, 0, 0, 0), |behaviour| behaviour.color);
                let color_data = color.as_rgba_u32();
                let mut color_data: Vec<u8> = vec![
                    ((color_data >> 0) & 0xff) as u8,
                    ((color_data >> 8) & 0xff) as u8,
                    ((color_data >> 16) & 0xff) as u8,
                    ((color_data >> 24) & 0xff) as u8,
                ];
                image_data.append(&mut color_data);
            }
        }
        image_data
    }

    pub fn create_collision_data(&self) -> Vec<Vec<Vec2>> {
        let mut islands: Vec<Island> = Vec::new();
        for i in 0..self.texels.len() {
            let local = texel_index_to_local(i);

            let edge_mask: u8 = if local.y == Chunk2D::SIZE.y - 1 {
                1 << 0
            } else {
                0
            } | if local.x == Chunk2D::SIZE.x - 1 {
                1 << 1
            } else {
                0
            } | if local.y == 0 { 1 << 2 } else { 0 }
                | if local.x == 0 { 1 << 3 } else { 0 };

            let mut sides: Vec<Segment2I>;
            let has_collision = TexelBehaviour2D::has_collision(&self.texels[i].id);
            if !has_collision {
                sides = MST_CASE_MAP[self.texels[i].neighbour_mask as usize]
                    .iter()
                    .clone()
                    .map(|side| Segment2I {
                        from: side.from + local,
                        to: side.to + local,
                    })
                    .collect();
            } else if has_collision && edge_mask != 0 {
                sides = Vec::with_capacity(Chunk2D::SIZE_X * 2 + Chunk2D::SIZE_Y * 2);
                for i in 0..MST_EDGE_CASE_MAP.len() {
                    if edge_mask & (1 << i) != 0 {
                        let edge = MST_EDGE_CASE_MAP[i];
                        sides.push(Segment2I {
                            from: edge.from + local,
                            to: edge.to + local,
                        })
                    }
                }
            } else {
                continue;
            }

            for side in sides {
                // Check if the side can be attached to any island
                // The naming of front and back are kind of misleading, and come from the VecDeque type.
                // You can think of the front as the beginning of the island loop, and back the end.

                // Connect to an island if possible, otherwise create a new island
                {
                    let mut connected_to: Option<&mut Island> = None;
                    for island in islands.iter_mut() {
                        if island.back().is_some() && island.back().unwrap().to == side.from {
                            connected_to = Some(island);
                        }
                    }

                    match connected_to {
                        Some(back) => {
                            back.push_back(side);
                        }
                        None => {
                            let mut island: Island = Island::new();
                            island.push_back(side);
                            islands.push(island);
                        }
                    }
                }

                // Find connected islands
                loop {
                    let mut merge_index: Option<usize> = None;
                    'outer: for i in 0..islands.len() {
                        for j in 0..islands.len() {
                            if i == j {
                                continue;
                            }
                            if islands[i].back().is_some()
                                && islands[j].front().is_some()
                                && islands[i].back().unwrap().to == islands[j].front().unwrap().from
                            {
                                merge_index = Some(i);
                                break 'outer;
                            }
                        }
                    }

                    // Merge connected islands
                    match merge_index {
                        Some(index) => {
                            let mut merge_from = islands.swap_remove(index);
                            match islands.iter_mut().find(|island| match island.front() {
                                Some(front) => front.from == merge_from.back().unwrap().to,
                                None => false,
                            }) {
                                Some(merge_to) => loop {
                                    match merge_from.pop_back() {
                                        Some(segment) => merge_to.push_front(segment),
                                        None => break,
                                    }
                                },
                                None => (),
                            };
                        }
                        None => break,
                    }
                }
            }
        }

        let mut result: Vec<Vec<Vec2>> = Vec::with_capacity(islands.len());
        for island in islands {
            if island.len() < 4 {
                continue;
            }
            let mut points: Vec<Vec2> = Vec::with_capacity(island.len() + 1);
            points.push(Vec2::from(island.front().unwrap().from));
            let mut current_angle: Option<f32> = None;
            for side in island {
                if current_angle.is_some() && (current_angle.unwrap() - side.angle()).abs() < 0.1 {
                    let len = points.len();
                    points[len - 1] = Vec2::from(side.to)
                } else {
                    current_angle = Some(side.angle());
                    points.push(Vec2::from(side.to));
                }
            }
            result.push(points);
        }
        result
    }
}

pub fn chunk_spawner(
    mut commands: Commands,
    mut terrain_events: EventReader<TerrainEvent2D>,
    mut images: ResMut<Assets<Image>>,
    chunk_query: Query<(Entity, &TerrainChunk2D)>,
) {
    for terrain_event in terrain_events.iter() {
        match terrain_event {
            TerrainEvent2D::ChunkAdded(chunk_index) => {
                // Create unique handle for the image
                let mut image = Image::new(
                    Extent3d {
                        width: Chunk2D::SIZE_X as u32,
                        height: Chunk2D::SIZE_Y as u32,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    vec![0x00; Chunk2D::SIZE_X * Chunk2D::SIZE_Y * 4],
                    bevy::render::render_resource::TextureFormat::Rgba8Unorm,
                );
                image.sampler_descriptor = ImageSampler::nearest();
                let texture = images.add(image);

                let pos = Vec2::from(*chunk_index * Chunk2D::SIZE);
                commands
                    .spawn(ChunkSpriteBundle {
                        chunk: TerrainChunk2D {
                            index: *chunk_index,
                        },
                        sprite: SpriteBundle {
                            sprite: Sprite {
                                custom_size: Some(Vec2::from(Chunk2D::SIZE)),
                                anchor: bevy::sprite::Anchor::BottomLeft,
                                ..default()
                            },
                            texture,
                            transform: Transform::from_translation(Vec3::new(pos.x, pos.y, 1.0)),
                            ..default()
                        },
                        ..default()
                    })
                    .insert(Name::new(format!(
                        "Chunk Sprite {},{}",
                        chunk_index.x, chunk_index.y
                    )));

                commands
                    .spawn(ChunkColliderBundle {
                        chunk: TerrainChunk2D {
                            index: *chunk_index,
                        },
                        transform: TransformBundle::from_transform(Transform::from_translation(
                            Vec3::new(pos.x, pos.y, 0.0),
                        )),
                        ..default()
                    })
                    .insert(Name::new(format!(
                        "Chunk Collider {},{}",
                        chunk_index.x, chunk_index.y
                    )));
            }
            TerrainEvent2D::ChunkRemoved(chunk_index) => {
                for (entity, chunk) in chunk_query.iter() {
                    if chunk.index == *chunk_index {
                        commands.entity(entity).despawn_recursive();
                    }
                }
            }
            _ => (),
        }
    }
}

/**
    Update the chunk sprite as needed
*/
pub fn chunk_sprite_sync(
    mut terrain_events: EventReader<TerrainEvent2D>,
    mut images: ResMut<Assets<Image>>,
    terrain: Res<Terrain2D>,
    added_chunk_query: Query<
        (Entity, &TerrainChunk2D),
        (With<TerrainChunkSpriteSync2D>, Changed<TerrainChunk2D>),
    >,
    chunk_query: Query<(Entity, &TerrainChunk2D), (With<TerrainChunkSpriteSync2D>, With<Sprite>)>,
    texture_query: Query<&Handle<Image>>,
) {
    let mut updated_chunks: Vec<(Entity, &TerrainChunk2D, Option<ChunkRect>)> = vec![];

    // Check for added components
    for (added_entity, added_chunk) in added_chunk_query.iter() {
        updated_chunks.push((added_entity, added_chunk, None));
    }

    // Check for terrain events
    for event in terrain_events.iter() {
        for (entity, chunk) in chunk_query.iter() {
            let (chunk_index, rect) = match event {
                TerrainEvent2D::ChunkAdded(chunk_index) => {
                    // The entity should not have the time to react to the event since it was just made
                    // REM: This gets called when new chunk is instantiated with brush
                    // println!("[chunk_sprite_sync -> TerrainEvent2D::ChunkAdded] This probably shouldn't be firing, maybe the chunk was destroyed and immediately created? chunk: {chunk_index:?}");
                    (chunk_index, None)
                }
                TerrainEvent2D::TexelsUpdated(chunk_index, rect) => (chunk_index, Some(*rect)),
                _ => continue,
            };

            if *chunk_index != chunk.index {
                continue;
            };

            updated_chunks.push((entity, chunk, rect));
        }
    }

    // Update sprite
    for (entity, chunk, rect) in updated_chunks {
        let chunk = terrain.index_to_chunk(&chunk.index).unwrap();
        // TODO: Update only the rect
        let _rect = rect.unwrap_or(ChunkRect {
            min: Vector2I::ZERO,
            max: Chunk2D::SIZE - Vector2I::ONE,
        });

        let handle = texture_query.get(entity).unwrap();
        let mut image = images.get_mut(handle).unwrap();
        let image_data = chunk.create_texture_data();
        image.data = image_data;
    }
}

/**
    Create and update colliders for chunk as needed
*/
pub fn chunk_collision_sync(
    mut terrain_events: EventReader<TerrainEvent2D>,
    mut commands: Commands,
    terrain: Res<Terrain2D>,
    added_chunk_query: Query<
        (Entity, &TerrainChunk2D),
        (With<TerrainChunkCollisionSync2D>, Changed<TerrainChunk2D>),
    >,
    chunk_query: Query<(Entity, &TerrainChunk2D), With<TerrainChunkCollisionSync2D>>,
    child_query: Query<&Children>,
    collider_query: Query<&Collider>,
) {
    let mut updated_chunks: Vec<(Entity, &TerrainChunk2D)> = vec![];

    // Check for added components
    for (added_entity, added_chunk) in added_chunk_query.iter() {
        updated_chunks.push((added_entity, added_chunk));
    }

    // Check for terrain events
    for event in terrain_events.iter() {
        for (entity, chunk) in chunk_query.iter() {
            let chunk_index = match event {
                TerrainEvent2D::ChunkAdded(chunk_index) => {
                    // The entity should not have the time to react to the event since it was just made
                    // REM: This gets called when new chunk is instantiated with brush
                    // println!("[chunk_collision_sync -> TerrainEvent2D::ChunkAdded] This probably shouldn't be firing, maybe the chunk was destroyed and immediately created? chunk: {chunk_index:?}");
                    chunk_index
                }
                TerrainEvent2D::TexelsUpdated(chunk_index, _) => chunk_index,
                _ => continue,
            };

            if *chunk_index != chunk.index {
                continue;
            };

            updated_chunks.push((entity, chunk));
        }
    }

    // let layer_membership = CollisionLayers::WORLD;

    // REM: Kinda messy, partly due do how entity creation is timed
    for (entity, chunk_component) in updated_chunks.iter() {
        let chunk = terrain.index_to_chunk(&chunk_component.index).unwrap();
        let new_islands = chunk.create_collision_data();

        // Create new colliders
        if let Ok(children) = child_query.get(*entity) {
            // Chunk has children, new ones will be created and old ones components will be removed
            for (index, island) in new_islands.iter().enumerate() {
                if let Some(child) = children.get(index) {
                    // Replace collider
                    commands
                        .entity(*child)
                        .insert(Collider::polyline(island.clone(), None));
                } else {
                    // Create new child
                    commands.entity(*entity).with_children(|builder| {
                        builder
                            .spawn(Collider::polyline(island.clone(), None))
                            .insert(TransformBundle::default())
                            .insert(CollisionGroups::new(CollisionLayers::WORLD, Group::ALL))
                            .insert(Name::new(format!("Island #{}", index)));
                    });
                }
            }
        } else {
            // Chunk doesn't have a Children component yet
            for (index, island) in new_islands.iter().enumerate() {
                commands.entity(*entity).with_children(|builder| {
                    builder
                        .spawn(Collider::polyline(island.clone(), None))
                        .insert(TransformBundle::default())
                        .insert(CollisionGroups::new(CollisionLayers::WORLD, Group::ALL))
                        .insert(Name::new(format!("Island #{}", index)));
                });
            }
        };

        // Remove extra children.
        // Leaving them seems to cause weird problems with rapier when re-adding the collider. The collider is ignored until something else is updated.
        for children in child_query.get(*entity) {
            for (index, child) in children.iter().enumerate() {
                if let Ok(_) = collider_query.get(*child) {
                    if index >= new_islands.len() {
                        commands.entity(*child).despawn_recursive();
                    }
                }
            }
        }
    }
}
