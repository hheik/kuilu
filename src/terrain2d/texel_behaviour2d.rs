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
            color: Color::rgb(0.0, 0.0, 1.0),
            form: TexelForm::Liquid,
            ..default()
        });
        
        result.insert(5, TexelBehaviour2D {
            color: Color::rgb(0.0, 1.0, 0.0),
            form: TexelForm::Gas,
            ..default()
        });

        result
    };
}

#[derive(Clone, Copy, Default, PartialEq)]
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

// TODO: change form-based functions like is_solid to behaviour based (e.g. has_collision) 
impl TexelBehaviour2D {
    pub fn from_id(id: &TexelID) -> Option<Self> {
        ID_MAP.get(id).copied()
    }

    pub fn is_empty(id: &TexelID) -> bool {
        ID_MAP.get(id).is_none()
    }

    pub fn is_solid(id: &TexelID) -> bool {
        ID_MAP.get(id).map_or(false, |tb| tb.form == TexelForm::Solid)
    }

    pub fn is_liquid(id: &TexelID) -> bool {
        ID_MAP.get(id).map_or(false, |tb| tb.form == TexelForm::Liquid)
    }

    pub fn is_gas(id: &TexelID) -> bool {
        ID_MAP.get(id).map_or(false, |tb| tb.form == TexelForm::Gas)
    }
}
