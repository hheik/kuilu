use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{
    terrain2d::{Chunk2D, Terrain2D, Terrain2DPlugin, TerrainGen2D},
    util::Vector2I,
};

use self::{
    camera::{GameCameraPlugin, WORLD_WIDTH},
    debug::DebugPlugin,
    kinematic::KinematicPlugin,
    player::PlayerPlugin,
};

pub mod camera;
pub mod debug;
pub mod kinematic;
pub mod player;

pub fn init() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(Terrain2DPlugin)
        .add_plugin(DebugPlugin)
        .add_plugin(KinematicPlugin)
        .add_plugin(GameCameraPlugin)
        .add_plugin(PlayerPlugin)
        .add_startup_system(setup_terrain)
        .add_startup_system(setup_window)
        .run();
}

fn setup_window(mut windows: ResMut<Windows>) {
    if let Some(window) = windows.get_primary_mut() {
        window.set_resolution(900.0, 450.0);
        window.set_title("Kuilu".to_string());
    }
}

fn setup_terrain(mut commands: Commands, mut terrain: ResMut<Terrain2D>) {
    let terrain_gen = TerrainGen2D::new(432678);
    for y in 0..(WORLD_WIDTH / Chunk2D::SIZE_Y as i32) {
        for x in 0..(WORLD_WIDTH / Chunk2D::SIZE_X as i32) {
            let position = Vector2I { x, y };
            terrain.add_chunk(position, terrain_gen.gen_chunk(&position));
        }
    }

    commands
        .spawn(Name::new("Left wall"))
        .insert(Collider::halfspace(Vec2::X).unwrap())
        .insert(TransformBundle::from_transform(
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ));

    commands
        .spawn(Name::new("Right wall"))
        .insert(Collider::halfspace(Vec2::NEG_X).unwrap())
        .insert(TransformBundle::from_transform(
            Transform::from_translation(Vec3::new(WORLD_WIDTH as f32, 0.0, 0.0)),
        ));
}
