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
    pub state: KinematicState,
    pub properties: KinematicProperties,
    pub rigidbody: RigidBody,
    pub events: ActiveEvents,
    pub collisions: ActiveCollisionTypes,
    pub transform: TransformBundle,
}

impl Default for KinematicBundle {
    fn default() -> Self {
        KinematicBundle {
            state: KinematicState::default(),
            properties: KinematicProperties::default(),
            rigidbody: RigidBody::KinematicPositionBased,
            events: ActiveEvents::COLLISION_EVENTS,
            collisions: ActiveCollisionTypes::all(),
            transform: TransformBundle::default(),
        }
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct KinematicState {
    // TODO: fork rapier2d to make it reflect?
    #[reflect(ignore)]
    pub last_move: Option<MoveShapeOutput>,
    pub did_jump: bool,
}

impl KinematicState {
    pub fn can_jump(&self) -> bool {
        self.last_move.as_ref().map_or(false, |last| last.grounded) && !self.did_jump
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
    pub gravity: Option<f32>,
}

impl Default for KinematicProperties {
    fn default() -> Self {
        Self {
            ground_speed: 75.0,
            ground_acceleration: 20.0,
            ground_friction: 30.0,
            air_speed: 75.0,
            air_acceleration: 10.0,
            air_friction: 10.0,
            jump_height: 100.0,
            gravity: Some(1.0),
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
    mut query: Query<(
        Entity,
        &mut KinematicState,
        &mut Transform,
        &KinematicProperties,
        &GlobalTransform,
        Option<&KinematicInput>,
        Option<&CollisionGroups>,
    )>,
    shape_query: Query<&Collider, Without<Sensor>>,
    child_query: Query<&Children>,
    mut rapier_context: ResMut<RapierContext>,
) {
    let dt = rapier_context.integration_parameters.dt;
    for (
        entity,
        mut kinematic_state,
        mut transform,
        props,
        global_transform,
        input,
        collision_groups,
    ) in query.iter_mut()
    {
        let default = &KinematicInput::default();
        let input = input.unwrap_or(default);

        let (speed, acceleration, friction) = if kinematic_state
            .last_move
            .as_ref()
            .map_or(false, |last| last.grounded)
        {
            (
                props.ground_speed,
                props.ground_acceleration,
                props.ground_friction,
            )
        } else {
            (props.air_speed, props.air_acceleration, props.air_friction)
        };

        const GRAVITY_DIR: Vec2 = Vec2::NEG_Y;
        const GRAVITY_COEFFICIENT: f32 = 2.0;

        let current_velocity = kinematic_state
            .last_move
            .as_ref()
            .map_or(Vec2::ZERO, |last| {
                if last.grounded {
                    last.effective_translation
                        .reject_from_normalized(GRAVITY_DIR)
                } else {
                    last.effective_translation
                }
            })
            / dt;
        let target_velocity = input.movement * speed;

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

        let mut velocity = if let Some(gravity) = props.gravity {
            // Also apply gravity
            move_towards_vec2(
                current_velocity,
                target_velocity.reject_from_normalized(GRAVITY_DIR)
                    + current_velocity.project_onto_normalized(GRAVITY_DIR),
                velocity_change_speed * dt,
            ) + GRAVITY_DIR * GRAVITY_COEFFICIENT * gravity
        } else {
            move_towards_vec2(
                current_velocity,
                target_velocity,
                velocity_change_speed * dt,
            )
        };

        if input.want_jump && kinematic_state.can_jump() {
            velocity = Vec2 {
                y: props.jump_height,
                ..velocity
            };
            kinematic_state.did_jump = true;
        }

        let shape = if let Ok(shape) = shape_query.get(entity) {
            Some(shape)
        } else if let Ok(children) = child_query.get(entity) {
            children
                .iter()
                .find_map(|child| shape_query.get(*child).ok())
        } else {
            None
        };

        // move
        kinematic_state.last_move = if let Some(shape) = shape {
            let (_scale, rotation, translation) = global_transform.to_scale_rotation_translation();

            let move_options = &MoveShapeOptions {
                up: Vec2::Y,
                autostep: Some(CharacterAutostep {
                    min_width: CharacterLength::Absolute(0.5),
                    max_height: CharacterLength::Absolute(2.1),
                    include_dynamic_bodies: false,
                }),
                slide: true,
                max_slope_climb_angle: (50.0_f32).to_radians(),
                min_slope_slide_angle: (50.0_f32).to_radians(),
                snap_to_ground: Some(CharacterLength::Absolute(5.0)),
                // snap_to_ground: props.gravity.map_or(None, |_| {
                //     if velocity.y <= 0.0 {
                //         Some(CharacterLength::Absolute(5.0))
                //     } else {
                //         None
                //     }
                // }),
                offset: CharacterLength::Absolute(0.01),
                ..MoveShapeOptions::default()
            };

            let mut filter = QueryFilter::new();
            let predicate = |coll_entity| coll_entity != entity;
            filter.predicate = Some(&predicate);

            if let Some(collision_groups) = collision_groups {
                filter.groups(InteractionGroups::new(
                    bevy_rapier2d::rapier::geometry::Group::from_bits_truncate(
                        collision_groups.memberships.bits(),
                    ),
                    bevy_rapier2d::rapier::geometry::Group::from_bits_truncate(
                        collision_groups.filters.bits(),
                    ),
                ));
            }

            let last_move: MoveShapeOutput = rapier_context.move_shape(
                velocity * dt,
                shape,
                translation.truncate(),
                rotation.to_euler(EulerRot::ZYX).0,
                shape.raw.0.mass_properties(1.0).mass(),
                move_options,
                filter,
                |_coll: CharacterCollision| (),
            );

            // Apply movement
            transform.translation += last_move.effective_translation.extend(0.0);

            Some(last_move)
        } else {
            None
        };

        if props.gravity.is_some() {
            // Reset any possible jump snapping and stuff after the peak of jump
            if velocity.y <= 0.0 {
                kinematic_state.did_jump = false;
            }
        }
    }
}
