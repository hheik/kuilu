use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::util::*;

pub struct KinematicPlugin;

impl Plugin for KinematicPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<KinematicProperties>()
            .register_type::<KinematicInput>()
            .add_system(kinematic_movement);
    }
}

#[derive(Bundle)]
pub struct KinematicBundle {
    pub rigidbody: RigidBody,
    pub velocity: Velocity,
    pub gravity_scale: GravityScale,
    pub collider: Collider,
    pub locked_axes: LockedAxes,
    pub events: ActiveEvents,
    pub collisions: ActiveCollisionTypes,
    pub properties: KinematicProperties,
    #[bundle]
    pub transform: TransformBundle,
}

impl Default for KinematicBundle {
    fn default() -> Self {
        KinematicBundle {
            rigidbody: RigidBody::Dynamic,
            gravity_scale: GravityScale(4.0),
            locked_axes: LockedAxes::ROTATION_LOCKED,
            events: ActiveEvents::COLLISION_EVENTS,
            collisions: ActiveCollisionTypes::all(),
            collider: Collider::default(),
            properties: KinematicProperties::default(),
            transform: TransformBundle::default(),
            velocity: Velocity::default(),
        }
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct KinematicProperties {
    pub ground_speed: f32,
    pub ground_acceleration: f32,
    pub ground_friction: f32,
    pub air_speed: f32,
    pub air_acceleration: f32,
    pub air_friction: f32,
}

impl Default for KinematicProperties {
    fn default() -> Self {
        Self {
            ground_speed: 100.0,
            ground_acceleration: 20.0,
            ground_friction: 30.0,
            air_speed: 100.0,
            air_acceleration: 10.0,
            air_friction: 10.0,
        }
    }
}

#[derive(Default, Component, Reflect)]
#[reflect(Component)]
pub struct KinematicInput {
    pub movement: Vec2,
}

fn kinematic_movement(
    time: Res<Time>,
    mut query: Query<(
        &mut Velocity,
        &KinematicProperties,
        Option<&KinematicInput>,
        Option<&GravityScale>,
    )>,
) {
    for (mut velocity, props, input, gravity) in query.iter_mut() {
        let default = &KinematicInput::default();
        let input = input.unwrap_or(default);

        let has_gravity = if let Some(gravity) = gravity {
            gravity.0.abs() < f32::EPSILON
        } else {
            false
        };
        let on_ground = if has_gravity { true } else { false };

        let (speed, acceleration, friction) = if on_ground {
            (
                props.ground_speed,
                props.ground_acceleration,
                props.ground_friction,
            )
        } else {
            (props.air_speed, props.air_acceleration, props.air_friction)
        };

        const GRAVITY_DIR: Vec2 = Vec2::NEG_Y;

        let current_velocity = velocity.linvel;
        let target_velocity =
            input.movement * speed + current_velocity.project_onto_normalized(GRAVITY_DIR);

        let angle_lerp = if current_velocity.length_squared() > 0.01 {
            let result = inverse_lerp(
                0.0,
                PI,
                current_velocity
                    .angle_between(target_velocity - current_velocity)
                    .abs(),
            );
            if result.is_nan() {
                0.0
            } else {
                result
            }
        } else {
            0.0
        };
        let delta_interpolation = angle_lerp.clamp(0.0, 1.0);
        let velocity_change_speed = lerp(acceleration, friction, delta_interpolation) * speed;

        velocity.linvel = move_towards_vec2(
            current_velocity,
            target_velocity,
            velocity_change_speed * time.delta_seconds(),
        );
    }
}
