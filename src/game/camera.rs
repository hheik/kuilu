use bevy::{prelude::*, render::camera::ScalingMode};

use crate::util::{move_towards_vec3, vec3_lerp};

pub struct GameCameraPlugin;

impl Plugin for GameCameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CameraFollow>()
            .add_startup_system(camera_setup)
            .add_system(camera_system);
    }
}

#[derive(Clone, Copy)]
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

#[derive(Default, Component, Reflect)]
#[reflect(Component)]
pub struct CameraFollow {
    pub priority: i32,
    #[reflect(ignore)]
    pub movement: FollowMovement,
}

fn camera_setup(mut commands: Commands) {
    commands
        .spawn()
        .insert(Name::new("Camera"))
        .insert_bundle(Camera2dBundle {
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::FixedHorizontal(320.0),
                ..default()
            },
            ..default()
        });
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
