use bevy::prelude::*;
use bevy_prototype_debug_lines::DebugLinesPlugin;

mod terrain;

use terrain::TerrainDebugPlugin;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(DebugLinesPlugin::default())
            // .add_plugin(bevy_rapier2d::prelude::RapierDebugRenderPlugin::default())
            // .add_plugin(bevy_inspector_egui::WorldInspectorPlugin::new())
            .add_plugin(TerrainDebugPlugin);
    }
}
