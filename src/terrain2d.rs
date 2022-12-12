use std::collections::{
    hash_map::{Iter, IterMut},
    HashMap,
};

use bevy::{prelude::*, render::camera::RenderTarget};
use bevy_prototype_debug_lines::DebugLines;

mod chunk2d;
mod terrain_gen2d;
mod texel2d;

pub use chunk2d::*;
pub use terrain_gen2d::*;
pub use texel2d::*;

use crate::{
    game::camera::GameCamera,
    util::{math::*, Vector2I},
};

pub struct Terrain2DPlugin;

impl Plugin for Terrain2DPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TerrainChunk2D>()
            .insert_resource(Terrain2D::new())
            .add_event::<TerrainEvent2D>()
            .add_system(debug_painter)
            .add_system_to_stage(
                CoreStage::PostUpdate,
                dirty_rect_visualizer.before(emit_terrain_events),
            )
            // DEBUG:
            .add_system_to_stage(CoreStage::First, first_log)
            .add_system_to_stage(CoreStage::Last, last_log)
            .add_system_to_stage(
                CoreStage::PostUpdate,
                chunk_spawner.before(emit_terrain_events),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                chunk_sprite_sync.after(chunk_spawner),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                chunk_collision_sync.after(chunk_spawner),
            )
            .add_system_to_stage(CoreStage::PostUpdate, emit_terrain_events);
    }
}

// DEBUG:
fn first_log() {
    println!("start <");
}

// DEBUG:
fn last_log(
    chunk_query: Query<(Entity, &TerrainChunk2D)>,
    child_query: Query<&Children>,
    mut commands: Commands,
) {
    println!("> end");
    for (entity, chunk) in chunk_query.iter() {
        println!("chunk! {entity:?} {:?}", chunk.index);
        for children in child_query.get(entity).iter() {
            for child in children.iter() {
                print!("\t");
                commands.entity(*child).log_components()
            }
        }
    }
}

fn debug_painter(
    mut terrain: ResMut<Terrain2D>,
    windows: Res<Windows>,
    input: Res<Input<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
) {
    if !input.pressed(MouseButton::Left) && !input.pressed(MouseButton::Right) {
        return;
    }

    // REM: Dirty and hopefully temporary
    // https://bevy-cheatbook.github.io/cookbook/cursor2world.html#2d-games
    // get the camera info and transform
    // assuming there is exactly one main camera entity, so query::single() is OK
    let (camera, camera_transform) = camera_query.single();

    // get the window that the camera is displaying to (or the primary window)
    let window = if let RenderTarget::Window(id) = camera.target {
        windows.get(id).unwrap()
    } else {
        windows.get_primary().unwrap()
    };

    // check if the cursor is inside the window and get its position
    let world_pos = if let Some(screen_pos) = window.cursor_position() {
        // get the size of the window
        let window_size = Vec2::new(window.width() as f32, window.height() as f32);

        // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
        let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;

        // matrix for undoing the projection and camera transform
        let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();

        // use it to convert ndc to world-space coordinates
        let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

        // reduce it to a 2D value
        world_pos.truncate()
    } else {
        return;
    };

    let origin = Vector2I::from(world_pos);
    let radius: i32 = 12;
    let id = match (
        input.pressed(MouseButton::Left),
        input.pressed(MouseButton::Right),
    ) {
        (true, false) => 1,
        (_, _) => 0,
    };

    for y in origin.y - (radius - 1)..origin.y + radius {
        for x in origin.x - (radius - 1)..origin.x + radius {
            let dx = (x - origin.x).abs();
            let dy = (y - origin.y).abs();
            if dx * dx + dy * dy <= (radius - 1) * (radius - 1) {
                terrain.set_texel(&Vector2I { x, y }, id)
            }
        }
    }
}

/**
    Visualize dirty rects
*/
fn dirty_rect_visualizer(terrain: Res<Terrain2D>, mut debug_draw: ResMut<DebugLines>) {
    for (chunk_index, chunk) in terrain.chunk_iter() {
        let rect = if let Some(rect) = chunk.dirty_rect {
            rect
        } else {
            continue;
        };

        let color = Color::RED;

        let points = vec![
            Vec3::new(rect.min.x as f32, rect.min.y as f32, 0.0),
            Vec3::new((rect.max.x + 1) as f32, rect.min.y as f32, 0.0),
            Vec3::new((rect.max.x + 1) as f32, (rect.max.y + 1) as f32, 0.0),
            Vec3::new(rect.min.x as f32, (rect.max.y + 1) as f32, 0.0),
        ];
        for i in 0..points.len() {
            let offset = Vec3::from(chunk_index_to_global(chunk_index));
            debug_draw.line_colored(
                offset + points[i],
                offset + points[(i + 1) % points.len()],
                0.0,
                color,
            );
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
            chunk.mark_clean();
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

    pub fn set_texel(&mut self, global: &Vector2I, id: TexelID) {
        let index = global_to_chunk_index(global);
        match self.index_to_chunk_mut(&index) {
            Some(chunk) => chunk.set_texel(&global_to_local(global), id),
            None => {
                let mut chunk = Chunk2D::new();
                chunk.set_texel(&global_to_local(global), id);
                self.add_chunk(index, chunk);
            }
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

pub fn global_to_local(position: &Vector2I) -> Vector2I {
    Vector2I {
        x: wrapping_remainder(position.x, Chunk2D::SIZE.x),
        y: wrapping_remainder(position.y, Chunk2D::SIZE.y),
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
