use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::util::*;

pub struct KinematicPlugin;

impl Plugin for KinematicPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<KinematicState>()
            .register_type::<KinematicProperties>()
            .register_type::<KinematicInput>()
            .add_system(kinematic_movement);
    }
}

#[derive(Bundle)]
pub struct KinematicBundle {
    pub kinematic: KinematicState,
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
            kinematic: KinematicState::default(),
            rigidbody: RigidBody::Dynamic,
            gravity_scale: GravityScale(3.0),
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

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct KinematicState {
    on_ground: bool,
    did_jump: bool,
    air_jump_counter: u8,
}

impl KinematicState {
    #[inline]
    pub fn on_ground(&self) -> bool {
        self.on_ground
    }
    #[inline]
    pub fn did_jump(&self) -> bool {
        self.did_jump
    }
    #[inline]
    pub fn air_jump_counter(&self) -> u8 {
        self.air_jump_counter
    }

    pub fn can_jump(&self) -> bool {
        if self.on_ground && !self.did_jump {
            return true;
        }
        false
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
    pub jump_height: f32,
    pub air_jumps: u8,
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
            jump_height: 150.0,
            air_jumps: 1,
        }
    }
}

#[derive(Default, Component, Reflect)]
#[reflect(Component)]
pub struct KinematicInput {
    pub movement: Vec2,
    pub want_jump: bool,
}

fn kinematic_movement(
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut Velocity,
        &mut KinematicState,
        &KinematicProperties,
        &GlobalTransform,
        Option<&KinematicInput>,
        Option<&GravityScale>,
        Option<&Collider>,
    )>,
    rapier_context: Res<RapierContext>,
) {
    for (
        entity,
        mut velocity,
        mut kinematic_state,
        props,
        global_transform,
        input,
        gravity,
        collider,
    ) in query.iter_mut()
    {
        let default = &KinematicInput::default();
        let input = input.unwrap_or(default);

        let has_gravity = if let Some(gravity) = gravity {
            gravity.0.abs() > f32::EPSILON
        } else {
            false
        };

        let (speed, acceleration, friction) = if kinematic_state.on_ground {
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

        if input.want_jump && kinematic_state.can_jump() {
            velocity.linvel = Vec2 {
                y: props.jump_height,
                ..velocity.linvel
            };
            kinematic_state.did_jump = true;
        }

        if has_gravity {
            // Reset any possible jump snapping and stuff after the peak of jump
            if velocity.linvel.y <= 0.0 {
                kinematic_state.did_jump = false;
            }
            if let Some(collider) = collider {
                let (_, rot, pos) = global_transform.to_scale_rotation_translation();
                // println!("{pos}");
                let angle = rot.to_euler(EulerRot::YXZ).2;
                let mut shape = collider.clone();
                shape.set_scale(Vec2::ONE * 0.9, 1);
                if let Some((_coll_entity, _hit)) = rapier_context.cast_shape(
                    Vec2::new(pos.x, pos.y),
                    angle,
                    Vec2::NEG_Y,
                    &shape,
                    2.0,
                    QueryFilter::new().exclude_collider(entity),
                ) {
                    // println!(
                    //     "Collision!\n\t{id:?} {name}\n\t{hit:?}",
                    //     id = coll_entity,
                    //     name = name_query.get(coll_entity).unwrap(),
                    // );
                    kinematic_state.on_ground = true;
                } else {
                    kinematic_state.on_ground = false;
                }
            }
        }
    }
}
