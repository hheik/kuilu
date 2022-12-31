use std::collections::HashMap;

use crate::{game::camera::GameCamera, terrain2d::*, util::Vector2I};
use bevy::{input::mouse::MouseWheel, prelude::*, render::camera::RenderTarget};
use bevy_prototype_debug_lines::DebugLines;

pub struct TerrainDebugPlugin;

impl Plugin for TerrainDebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TerrainBrush2D::default())
            // .add_system_to_stage(TerrainStages::EventHandler, dirty_rect_visualizer)
            // .add_system_to_stage(CoreStage::Last, chunk_debugger)
            .add_system(debug_painter);
    }
}

#[derive(Resource)]
struct TerrainBrush2D {
    pub radius: i32,
    pub tile: TexelID,
}

impl Default for TerrainBrush2D {
    fn default() -> Self {
        TerrainBrush2D {
            radius: 40,
            tile: 8,
        }
    }
}

// REM: Dirty and hopefully temporary
fn debug_painter(
    mut terrain: ResMut<Terrain2D>,
    mut debug_draw: ResMut<DebugLines>,
    mut brush: ResMut<TerrainBrush2D>,
    windows: Res<Windows>,
    mouse_input: Res<Input<MouseButton>>,
    key_input: Res<Input<KeyCode>>,
    mut mouse_wheel: EventReader<MouseWheel>,
    camera_query: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
) {
    // let allow_painting = key_input.pressed(KeyCode::LControl);
    let allow_painting = true;

    // Change brush
    for event in mouse_wheel.iter() {
        if allow_painting {
            brush.radius = (brush.radius + event.y.round() as i32).clamp(1, 128);
        }
    }

    if !allow_painting {
        return;
    }

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

    for (index, key) in vec![
        KeyCode::Key1,
        KeyCode::Key2,
        KeyCode::Key3,
        KeyCode::Key4,
        KeyCode::Key5,
        KeyCode::Key6,
        KeyCode::Key7,
        KeyCode::Key8,
        KeyCode::Key9,
    ]
    .iter()
    .enumerate()
    {
        if key_input.just_pressed(*key) {
            brush.tile = index as u8 + 1;
        }
    }

    let origin = Vector2I::from(world_pos);
    let radius = brush.radius;
    let id = match (
        mouse_input.pressed(MouseButton::Left),
        mouse_input.pressed(MouseButton::Right),
    ) {
        (true, false) => brush.tile,
        (_, _) => 0,
    };
    let color = TexelBehaviour2D::from_id(&brush.tile)
        .map_or(Color::rgba(0.0, 0.0, 0.0, 0.0), |tb| tb.color);

    for y in origin.y - (radius - 1)..origin.y + radius {
        for x in origin.x - (radius - 1)..origin.x + radius {
            let dx = (x - origin.x).abs();
            let dy = (y - origin.y).abs();
            let draw = dx * dx + dy * dy <= (radius - 1) * (radius - 1);

            if draw {
                let pos: Vector2I = Vector2I { x, y };
                debug_draw.line_colored(
                    Vec3::from(pos) + Vec3::new(0.45, 0.45, 1.0),
                    Vec3::from(pos) + Vec3::new(0.55, 0.55, 1.0),
                    0.0,
                    color,
                );
                if mouse_input.pressed(MouseButton::Left) || mouse_input.pressed(MouseButton::Right)
                {
                    terrain.set_texel(&pos, Texel2D { id, ..default() }, None)
                }
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

        let offset = Vec3::from(chunk_index_to_global(chunk_index));
        let min = offset + Vec3::from(rect.min);
        let max = offset + Vec3::from(rect.max + Vector2I::ONE);
        draw_box(&mut debug_draw, min, max, Color::RED, 0.0);
    }
}

fn chunk_debugger(terrain: Res<Terrain2D>, mut debug_draw: ResMut<DebugLines>) {
    for (chunk_index, chunk) in terrain.chunk_iter() {
        println!("chunk contents: {chunk_index:?}");
        let offset = Vec3::from(chunk_index_to_global(chunk_index));
        let min = offset + Vec3::ZERO;
        let max = offset + Vec3::from(Chunk2D::SIZE);
        draw_box(
            &mut debug_draw,
            min,
            max,
            Color::rgba(0.5, 0.0, 0.5, 0.5),
            0.0,
        );

        let mut tile_counter: HashMap<TexelID, (u32, u32)> = HashMap::new();
        for y in 0..Chunk2D::SIZE_Y as i32 {
            for x in 0..Chunk2D::SIZE_X as i32 {
                let local = Vector2I::new(x, y);
                let global = local_to_global(&local, chunk_index);
                if let (Some(texel), _) = terrain.get_texel_behaviour(&global) {
                    if !tile_counter.contains_key(&texel.id) {
                        tile_counter.insert(texel.id, (0, 0));
                    }
                    let (old_count, old_density) = tile_counter[&texel.id].clone();
                    tile_counter.insert(
                        texel.id,
                        (old_count + 1, old_density + texel.density as u32),
                    );
                }
            }
        }

        let mut counts: Vec<(u8, String, u32, u32)> = vec![];

        for (id, (count, total_density)) in tile_counter.iter() {
            let name =
                TexelBehaviour2D::from_id(id).map_or("unknown".to_string(), |b| b.name.to_string());
            counts.push((*id, name, *count, *total_density));
        }
        counts.sort_unstable_by_key(|c| c.0);
        for (id, name, count, total_density) in counts {
            println!(
                "\tmaterial: {name:<24}id: {id:<8}count: {count:<8}total_density: {total_density:<8}"
            );
        }
    }
}

pub fn draw_box(debug_draw: &mut DebugLines, min: Vec3, max: Vec3, color: Color, duration: f32) {
    let points = vec![
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
    ];
    for i in 0..points.len() {
        debug_draw.line_colored(points[i], points[(i + 1) % points.len()], duration, color);
    }
}
