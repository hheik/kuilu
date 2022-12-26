use crate::util::Vector2I;

use super::TexelID;
use bevy::prelude::*;
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref ID_MAP: HashMap<TexelID, TexelBehaviour2D> = {
        let mut result = HashMap::new();

        result.insert(
            1,
            TexelBehaviour2D {
                name: String::from("loose sand"),
                color: Color::rgb(0.61, 0.49, 0.38),
                gravity: Some(TexelGravity::Down(100)),
                has_collision: true,
                ..default()
            },
        );

        result.insert(
            2,
            TexelBehaviour2D {
                name: String::from("loose stone"),
                color: Color::rgb(0.21, 0.19, 0.17),
                gravity: Some(TexelGravity::Down(100)),
                has_collision: true,
                ..default()
            },
        );

        result.insert(
            3,
            TexelBehaviour2D {
                name: String::from("loose sturdy stone"),
                color: Color::rgb(0.11, 0.11, 0.11),
                gravity: Some(TexelGravity::Down(100)),
                has_collision: true,
                ..default()
            },
        );

        result.insert(
            4,
            TexelBehaviour2D {
                name: String::from("water"),
                color: Color::rgba(0.0, 0.0, 1.0, 0.5),
                form: TexelForm::Liquid,
                gravity: Some(TexelGravity::Down(10)),
                ..default()
            },
        );

        result.insert(
            5,
            TexelBehaviour2D {
                name: String::from("oil"),
                color: Color::rgba(0.0, 1.0, 0.0, 0.5),
                form: TexelForm::Gas,
                gravity: Some(TexelGravity::Up(50)),
                ..default()
            },
        );

        result.insert(
            6,
            TexelBehaviour2D {
                name: String::from("gas"),
                color: Color::rgba(0.5, 0.5, 0.25, 0.5),
                form: TexelForm::Liquid,
                gravity: Some(TexelGravity::Down(5)),
                ..default()
            },
        );

        result.insert(
            11,
            TexelBehaviour2D {
                name: String::from("sand"),
                color: Color::rgb(0.61, 0.49, 0.38),
                has_collision: true,
                ..default()
            },
        );

        result.insert(
            12,
            TexelBehaviour2D {
                name: String::from("stone"),
                color: Color::rgb(0.21, 0.19, 0.17),
                has_collision: true,
                ..default()
            },
        );

        result.insert(
            13,
            TexelBehaviour2D {
                name: String::from("sturdy stone"),
                color: Color::rgb(0.11, 0.11, 0.11),
                has_collision: true,
                ..default()
            },
        );

        result.insert(
            u8::MAX,
            TexelBehaviour2D {
                color: Color::BLACK,
                has_collision: true,
                ..default()
            },
        );

        result
    };
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum TexelForm {
    #[default]
    // Solid materials, when affected by gravity, create pyramid-like piles
    Solid,
    // Liquid materials, when affected by gravity, act like solids but also try to stabilise the surface level by traveling flat surfaces
    Liquid,
    // Gas materials act like liquids, but also have density/pressure that causes them to disperse
    Gas,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TexelGravity {
    Down(u8),
    Up(u8),
}

impl From<TexelGravity> for Vector2I {
    fn from(gravity: TexelGravity) -> Self {
        match gravity {
            TexelGravity::Down(_) => Vector2I::DOWN,
            TexelGravity::Up(_) => Vector2I::UP,
        }
    }
}

#[derive(Clone)]
pub struct TexelBehaviour2D {
    pub name: String,
    pub color: Color,
    pub form: TexelForm,
    pub has_collision: bool,
    pub gravity: Option<TexelGravity>,
    pub toughness: Option<f32>,
}

impl Default for TexelBehaviour2D {
    fn default() -> Self {
        TexelBehaviour2D {
            name: "Unnamed material".to_string(),
            color: Color::PINK,
            form: TexelForm::Solid,
            has_collision: false,
            gravity: None,
            toughness: None,
        }
    }
}

// TODO: change form-based functions like is_solid to behaviour based (e.g. has_collision)
impl TexelBehaviour2D {
    pub fn from_id(id: &TexelID) -> Option<Self> {
        ID_MAP.get(id).cloned()
    }

    pub fn is_empty(id: &TexelID) -> bool {
        ID_MAP.get(id).is_none()
    }

    pub fn has_collision(id: &TexelID) -> bool {
        ID_MAP.get(id).map_or(false, |b| b.has_collision)
    }

    pub fn can_displace(from: &TexelBehaviour2D, to: &Option<TexelBehaviour2D>) -> bool {
        let to = if let Some(to) = to { to } else { return true };

        match (from.form, to.form) {
            (_, TexelForm::Solid) => false,
            (_, _) => {
                if let (Some(from_grav), Some(to_grav)) = (from.gravity, to.gravity) {
                    match (from_grav, to_grav) {
                        (TexelGravity::Down(from_grav), TexelGravity::Down(to_grav)) => {
                            from_grav > to_grav
                        }
                        (TexelGravity::Up(from_grav), TexelGravity::Up(to_grav)) => {
                            from_grav > to_grav
                        }
                        (_, _) => true,
                    }
                } else {
                    true
                }
            }
        }
    }
}
