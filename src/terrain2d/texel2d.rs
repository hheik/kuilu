pub use u8 as TexelID;

use super::TexelBehaviour2D;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Texel2D {
    /// Identifier for a set of properties
    pub id: TexelID,
    /// Used by gas materials
    pub density: u8,
}

impl Default for Texel2D {
    fn default() -> Self {
        Self {
            id: TexelID::default(),
            density: u8::MAX,
        }
    }
}

impl Texel2D {
    pub const EMPTY: TexelID = 0;

    pub fn has_collision(&self) -> bool {
        TexelBehaviour2D::has_collision(&self.id)
    }

    pub fn behaviour(&self) -> Option<TexelBehaviour2D> {
        TexelBehaviour2D::from_id(&self.id)
    }
}
