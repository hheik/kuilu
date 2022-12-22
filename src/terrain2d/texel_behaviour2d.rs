use super::TexelID;
use bevy::prelude::*;
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref ID_MAP: HashMap<TexelID, TexelBehaviour2D> = {
        let mut result = HashMap::new();

        result.insert(1, TexelBehaviour2D {
            color: Color::rgb(0.61, 0.49, 0.38),
            ..default()
        });

        result.insert(2, TexelBehaviour2D {
            color: Color::rgb(0.21, 0.19, 0.17),
            ..default()
        });

        result.insert(3, TexelBehaviour2D {
            color: Color::rgb(0.11, 0.11, 0.11),
            ..default()
        });

        result.insert(4, TexelBehaviour2D {
            color: Color::rgb(1.0, 0.0, 0.0),
            form: TexelForm::Gas,
            ..default()
        });

        result
    };
}

#[derive(Clone, Copy, Default)]
pub enum TexelForm {
    #[default]
    Solid,
    Liquid,
    Gas,
}

#[derive(Clone, Copy, Default)]
pub struct TexelBehaviour2D {
    // pub flammability: Option<f32>,
    // pub gravity: Option<f32>,
    pub form: TexelForm,
    pub color: Color,
}

impl TexelBehaviour2D {
    pub fn from_id(id: &TexelID) -> Option<Self> {
        ID_MAP.get(id).copied()
    }
}
