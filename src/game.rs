use bevy::prelude::*;
use bevy_inspector_egui::*;
use bevy_rapier2d::prelude::*;

use self::{camera::GameCameraPlugin, kinematic::KinematicPlugin, player::PlayerPlugin};

pub mod camera;
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
        .add_plugin(PlayerPlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands) {
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
            transform: Transform::from_xyz(0.0, -100.0, 0.0),
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
