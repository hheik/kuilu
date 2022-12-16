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
    // TODO: fork rapier2d and make it reflect?
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
            ground_speed: 100.0,
            ground_acceleration: 20.0,
            ground_friction: 30.0,
            air_speed: 100.0,
            air_acceleration: 10.0,
            air_friction: 10.0,
            jump_height: 150.0,
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
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut KinematicState,
        &mut Transform,
        &KinematicProperties,
        &GlobalTransform,
        Option<&KinematicInput>,
    )>,
    shape_query: Query<&Collider, Without<Sensor>>,
    child_query: Query<&Children>,
    mut rapier_context: ResMut<RapierContext>,
) {
    for (entity, mut kinematic_state, mut transform, props, global_transform, input) in
        query.iter_mut()
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

        let current_velocity = kinematic_state
            .last_move
            .as_ref()
            .map_or(Vec2::ZERO, |last| last.effective_translation);
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

        let mut velocity = move_towards_vec2(
            current_velocity,
            target_velocity,
            velocity_change_speed * time.delta_seconds(),
        );

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

        // gravity
        if let Some(gravity) = props.gravity {
            velocity.y += -9.81 * gravity * time.delta_seconds();
        }

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
                offset: CharacterLength::Absolute(0.01),
                ..MoveShapeOptions::default()
            };

            let last_move = rapier_context.move_shape(
                velocity * time.delta_seconds(),
                shape,
                translation.truncate(),
                rotation.to_euler(EulerRot::ZYX).0,
                shape.raw.0.mass_properties(1.0).mass(),
                move_options,
                QueryFilter::new(),
                |coll: CharacterCollision| println!("Collided! {coll:?}"),
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
            // if let Some(collider) = collider {
            //     let (_, rot, pos) = global_transform.to_scale_rotation_translation();
            //     let angle = rot.to_euler(EulerRot::YXZ).2;
            //     let mut shape = collider.clone();
            //     shape.set_scale(Vec2::ONE * 0.9, 1);
            //     if let Some((_coll_entity, _hit)) = rapier_context.cast_shape(
            //         Vec2::new(pos.x, pos.y),
            //         angle,
            //         Vec2::NEG_Y,
            //         &shape,
            //         2.0,
            //         QueryFilter::new().exclude_collider(entity),
            //     ) {
            //         kinematic_state.on_ground = true;
            //     } else {
            //         kinematic_state.on_ground = false;
            //     }
            // }
        }
    }
}
