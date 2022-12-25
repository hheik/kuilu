use bevy::prelude::*;

pub struct FrameCounterPlugin;

impl Plugin for FrameCounterPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FrameCounter { frame: 0 })
            .add_system_to_stage(CoreStage::First, frame_increment);
    }
}

#[derive(Resource)]
pub struct FrameCounter {
    pub frame: u64,
}

fn frame_increment(mut frame_counter: ResMut<FrameCounter>) {
    frame_counter.frame += 1;
}
