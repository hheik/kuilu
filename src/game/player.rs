use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use super::{
    camera::{CameraFollow, FollowMovement},
    kinematic::*,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PlayerInput>()
            .add_startup_system(player_spawn)
            .add_system(player_system);
    }
}

#[derive(Default, Component, Reflect)]
#[reflect(Component)]
pub struct PlayerInput;

#[derive(Default, Bundle)]
pub struct PlayerBundle {
    pub control: PlayerInput,
    #[bundle]
    pub kinematic: KinematicBundle,
}

pub fn player_system(
    input: Res<Input<KeyCode>>,
    mut query: Query<(&mut KinematicInput, &Transform), With<PlayerInput>>,
) {
    let (mut kinematic_input, _transform) = match query.get_single_mut() {
        Ok(single) => single,
        Err(_) => return,
    };

    let movement = Vec2 {
        x: input_to_axis(input.pressed(KeyCode::A), input.pressed(KeyCode::D)),
        // y: input_to_axis(input.pressed(KeyCode::S), input.pressed(KeyCode::W)),
        y: 0.0,
    };

    kinematic_input.movement = movement;
    kinematic_input.want_jump = input.pressed(KeyCode::Space)
}

fn input_to_axis(negative: bool, positive: bool) -> f32 {
    if negative == positive {
        return 0.0;
    }
    if negative {
        -1.0
    } else {
        1.0
    }
}

pub fn player_spawn(mut commands: Commands) {
    let kinematic = KinematicBundle {
        collider: Collider::cuboid(8.0, 16.0),
        transform: TransformBundle::default(),
        ..default()
    };

    commands
        .spawn(())
        .insert(Name::new("Player"))
        .insert(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.75, 0.25, 0.25),
                custom_size: Some(Vec2 { x: 16.0, y: 32.0 }),
                ..default()
            },
            ..default()
        })
        .insert(PlayerBundle {
            kinematic,
            ..default()
        })
        .insert(KinematicInput::default())
        .insert(CameraFollow {
            priority: 0,
            movement: FollowMovement::Smooth(7.0),
        });
}
