use bevy_rapier2d::prelude::*;

pub struct CollisionLayers;

impl CollisionLayers {
    pub const WORLD: Group = Group::GROUP_1;
    pub const PLAYER: Group = Group::GROUP_2;
    pub const ENEMY: Group = Group::GROUP_3;
}
