use bevy::{input::mouse::MouseWheel, prelude::*};
use bevy_inspector_egui::*;
use bevy_prototype_debug_lines::DebugLinesPlugin;
use bevy_rapier2d::prelude::*;

use crate::{
    terrain2d::{Chunk2D, Terrain2D, Terrain2DPlugin, TerrainGen2D},
    util::Vector2I,
};

use self::{
    camera::{CameraFollow, GameCameraPlugin, WORLD_WIDTH},
    kinematic::KinematicPlugin,
    player::PlayerPlugin,
};

pub mod camera;
pub mod kinematic;
pub mod player;

pub fn init() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(DebugLinesPlugin::default())
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(WorldInspectorPlugin::new())
        .add_plugin(KinematicPlugin)
        .add_plugin(GameCameraPlugin)
        .add_plugin(Terrain2DPlugin)
        .add_plugin(PlayerPlugin)
        .add_startup_system(setup_debug_terrain)
        .add_system(debug_controls)
        .run();
}

fn debug_controls(
    mut query: Query<&mut Transform, With<CameraFollow>>,
    mut events: EventReader<MouseWheel>,
) {
    for event in events.iter() {
        for mut transform in query.iter_mut() {
            transform.translation += Vec3::new(0.0, event.y, 0.0) * 30.0;
        }
    }
}

fn setup_debug_terrain(mut commands: Commands, mut terrain: ResMut<Terrain2D>) {
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
