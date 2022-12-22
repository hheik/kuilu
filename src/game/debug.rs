use bevy::prelude::*;
use bevy_inspector_egui::*;
use bevy_prototype_debug_lines::DebugLinesPlugin;
use bevy_rapier2d::prelude::*;

mod terrain;

use terrain::TerrainDebugPlugin;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(DebugLinesPlugin::default())
            .add_plugin(RapierDebugRenderPlugin::default())
            .add_plugin(WorldInspectorPlugin::new())
            .add_plugin(TerrainDebugPlugin);
    }
}
