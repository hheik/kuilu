use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};

use crate::util::{move_towards_vec3, vec3_lerp};

pub struct GameCameraPlugin;

impl Plugin for GameCameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_inspectable::<CameraFollow>()
            .add_startup_system(camera_setup)
            .add_system_to_stage(CoreStage::PostUpdate, camera_system);
    }
}

#[derive(Clone, Copy, Inspectable, PartialEq, Reflect)]
pub enum FollowMovement {
    Instant,
    Linear(f32),
    Smooth(f32),
}

impl Default for FollowMovement {
    fn default() -> Self {
        Self::Instant
    }
}

#[derive(Default, Component, Reflect, Inspectable)]
#[reflect(Component)]
pub struct CameraFollow {
    pub priority: i32,
    pub movement: FollowMovement,
}

fn camera_setup(mut commands: Commands) {
    commands.spawn((
        Name::new("Camera"),
        Camera2dBundle {
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::FixedHorizontal(320.0),
                ..default()
            },
            ..default()
        },
    ));
}

fn camera_system(
    time: Res<Time>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    follow_query: Query<(&Transform, &CameraFollow), Without<Camera2d>>,
) {
    let (target, follow) = match follow_query
        .iter()
        .max_by_key(|(_transform, follow)| follow.priority)
    {
        Some(followed) => followed,
        None => return,
    };

    for mut camera_transform in camera_query.iter_mut() {
        match follow.movement {
            FollowMovement::Instant => {
                camera_transform.translation = target.translation * Vec3::new(0.0, 1.0, 1.0)
            }
            FollowMovement::Linear(speed) => {
                camera_transform.translation = move_towards_vec3(
                    camera_transform.translation,
                    target.translation * Vec3::new(0.0, 1.0, 1.0),
                    speed * time.delta_seconds(),
                )
            }
            FollowMovement::Smooth(speed) => {
                camera_transform.translation = vec3_lerp(
                    camera_transform.translation,
                    target.translation * Vec3::new(0.0, 1.0, 1.0),
                    speed * time.delta_seconds(),
                )
            }
        }
    }
}
