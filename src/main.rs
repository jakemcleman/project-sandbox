#![deny(clippy::all)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::mem::size_of;

use bevy::{prelude::*, window::PresentMode, window::WindowResolution};

mod camera;
mod perf;
mod sandsim;
mod ui;
mod worldgen;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
enum UpdateStages {
    UI,
    Input,
    WorldUpdate,
    WorldDraw,
}

fn main() {
    println!("particle size: {}", size_of::<sandworld::Particle>());

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Project Sandbox - Bevy".to_string(),
                        resolution: WindowResolution::new(1920., 1080.),
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugin(crate::sandsim::SandSimulationPlugin)
        .add_plugin(crate::camera::CameraPlugin)
        .add_plugin(crate::ui::UiPlugin)
        .add_plugin(crate::perf::PerfControlPlugin)
        .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .run();
}
