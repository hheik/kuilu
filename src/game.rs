use bevy::{input::mouse::MouseWheel, prelude::*};
use bevy_inspector_egui::*;
use bevy_rapier2d::prelude::*;

use crate::{
    terrain2d::{Chunk2D, Terrain2D, Terrain2DPlugin, TerrainGen2D},
    util::Vector2I,
};

use self::{
    camera::{CameraFollow, GameCameraPlugin},
    kinematic::KinematicPlugin,
    player::PlayerPlugin,
};

pub mod camera;
pub mod chunk;
pub mod kinematic;
pub mod player;

pub fn init() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(WorldInspectorPlugin::new())
        .add_plugin(KinematicPlugin)
        .add_plugin(GameCameraPlugin)
        .add_plugin(Terrain2DPlugin)
        // .add_plugin(PlayerPlugin)
        .add_startup_system(setup_debug_terrain)
        .add_startup_system(setup_debug_camera)
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

fn setup_debug_camera(mut commands: Commands) {
    commands
        .spawn(TransformBundle::default())
        .insert(Name::new("Debug Camera"))
        .insert(CameraFollow {
            movement: camera::FollowMovement::Smooth(10.0),
            ..default()
        });
}

fn setup_debug_terrain(mut terrain: ResMut<Terrain2D>) {
    let terrain_gen = TerrainGen2D::new(432678);
    for y in 0..32 {
        for x in 0..8 {
            let position = Vector2I { x, y };
            terrain.add_chunk(position, terrain_gen.gen_chunk(&position));
        }
    }
}
