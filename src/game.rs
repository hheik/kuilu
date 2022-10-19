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
        .spawn()
        .insert(Name::new("Ground"))
        .insert(Collider::cuboid(400.0, 25.0))
        .insert_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.75),
                custom_size: Some(Vec2::new(800.0, 50.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, -100.0, 0.0),
            ..default()
        });
}
