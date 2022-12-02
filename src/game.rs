use bevy::prelude::*;
use bevy_inspector_egui::*;
use bevy_rapier2d::prelude::*;

use crate::{
    terrain2d::{Chunk2D, Terrain2D, Terrain2DPlugin},
    util::Vector2I,
};

use self::{camera::GameCameraPlugin, kinematic::KinematicPlugin, player::PlayerPlugin};

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
        // .add_startup_system(setup_debug_ground)
        .add_startup_system(setup_debug_terrain)
        .run();
}

fn setup_debug_terrain(mut terrain: ResMut<Terrain2D>) {
    for y in 0..32 {
        for x in 0..8 {
            terrain.add_chunk(Vector2I { x, y }, Chunk2D::new_circle());
        }
    }
}

fn setup_debug_ground(mut commands: Commands) {
    // Static ground
    commands
        .spawn(())
        .insert(Name::new("Ground"))
        .insert(Collider::cuboid(40.0, 25.0))
        .insert(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.75),
                custom_size: Some(Vec2::new(80.0, 50.0)),
                ..default()
            },
            transform: Transform::from_xyz(-100.0, -250.0, 0.0),
            ..default()
        });
    commands
        .spawn(())
        .insert(Name::new("Ground"))
        .insert(Collider::cuboid(40.0, 25.0))
        .insert(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.75),
                custom_size: Some(Vec2::new(80.0, 50.0)),
                ..default()
            },
            transform: Transform::from_xyz(100.0, -200.0, 0.0),
            ..default()
        });
    commands
        .spawn(())
        .insert(Name::new("Ground"))
        .insert(Collider::cuboid(100.0, 25.0))
        .insert(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.75),
                custom_size: Some(Vec2::new(200.0, 50.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, -300.0, 0.0),
            ..default()
        });
}
