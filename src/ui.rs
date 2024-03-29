use crate::sandsim::{BrushMode, BrushOptions};
use crate::chunk_display::DrawOptions;
use bevy::prelude::*;
use sandworld::ParticleType;

pub struct UiPlugin;

#[derive(Resource)]
pub struct PointerCaptureState {
    pub click_consumed: bool,
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_buttons)
            .add_systems(Startup, spawn_performance_info_text)
            .insert_resource(PointerCaptureState {
                click_consumed: false,
            })
            .add_systems(
                Update,
                button_system
                    .in_set(crate::UpdateStages::UI)
                    .before(crate::UpdateStages::Input),
            )
            .add_systems(
                Update,
                update_performance_text
                    .in_set(crate::UpdateStages::UI)
                    .after(crate::UpdateStages::WorldUpdate),
            );
    }
}

#[derive(Component)]
struct PerformanceReadout;

fn spawn_performance_info_text(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(
            TextBundle::from_sections([
                TextSection {
                    value: "FPS: 69".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                        font_size: 30.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                },
                TextSection {
                    value: "\nLoaded Regions: 000".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                        font_size: 20.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                },
                TextSection {
                    value: "\nSleeping Regions: 000".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                        font_size: 20.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                },
                TextSection {
                    value: "\nUpdated Regions: 000".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                        font_size: 20.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                },
                TextSection {
                    value: "\nChunk Updates: 000".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                        font_size: 20.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                },
                TextSection {
                    value: "\nAvg time per chunk".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                        font_size: 20.0,
                        color: Color::rgb(0.9, 0.9, 0.6),
                    },
                },
                TextSection {
                    value: "\nAvg render time per chunk".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                        font_size: 20.0,
                        color: Color::rgb(0.9, 0.9, 0.6),
                    },
                },
                TextSection {
                    value: "\nAvg chunk culling time".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                        font_size: 20.0,
                        color: Color::rgb(0.9, 0.9, 0.6),
                    },
                },
            ])
            .with_style(Style {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                ..Default::default()
            }),
        )
        .insert(PerformanceReadout {});
}

fn update_performance_text(
    mut text_query: Query<(&PerformanceReadout, &mut Text, &mut Visibility)>,
    stats: Res<crate::sandsim::WorldStats>,
    draw_options: Res<DrawOptions>,
    frame_times: Res<crate::perf::FrameTimes>,
) {
    let (_, mut text, mut vis) = text_query.single_mut();
    if draw_options.world_stats {
        *vis = Visibility::Inherited;

        text.sections[0].value = format!(
            "FPS: {} ({:.1}ms (worst: {:.1}ms))",
            (1. / frame_times.current_avg).round() as u32,
            frame_times.current_avg * 1000.,
            frame_times.recent_worst * 1000.,
        );

        if let Some(world_stats) = &stats.update_stats {
            text.sections[1].value = format!("\nLoaded Regions: {0} ({1}) [Compressed {2} ({3})]", 
                world_stats.loaded_regions, world_stats.loading_regions, 
                world_stats.compressed_regions, world_stats.compressing_regions);
            text.sections[2].value = format!("\nMouse position: {0} (Chunk: {1} Region: {2})", stats.mouse_grid_pos, stats.mouse_chunk_pos, stats.mouse_region);
            text.sections[3].value = format!("\nRegion Updates: {}", world_stats.region_updates);
            text.sections[4].value = format!(
                "\nChunk Updates [Target]: {} [{}]",
                world_stats.chunk_updates, stats.target_chunk_updates
            );

            if stats.chunk_texture_update_time.len() > 0 {
                let mut texture_update_time_avg = 0.;
                let mut texture_update_per_chunk_avg = 0.;
                for (time, count) in &stats.chunk_texture_update_time {
                    texture_update_time_avg += time;
                    texture_update_per_chunk_avg += time / (*count as f64);
                }
                texture_update_time_avg =
                    texture_update_time_avg / (stats.chunk_texture_update_time.len() as f64);
                texture_update_per_chunk_avg =
                    texture_update_per_chunk_avg / (stats.chunk_texture_update_time.len() as f64);

                text.sections[6].value = format!(
                    "\nTex Update time:  {:.2}ms - Avg time per chunk: {:.3}ms",
                    texture_update_time_avg * 1000.,
                    texture_update_per_chunk_avg * 1000.
                );
            }
            if stats.chunk_cull_time.len() > 0 {
                let mut cull_time_avg = 0.;
                let mut culled_chunks_avg = 0;
                for (time, count) in &stats.chunk_cull_time {
                    cull_time_avg += time;
                    culled_chunks_avg += count;
                }
                cull_time_avg =
                    cull_time_avg / (stats.chunk_cull_time.len() as f64);

                culled_chunks_avg =
                    culled_chunks_avg / stats.chunk_cull_time.len() as u64;
    
                text.sections[7].value = format!(
                    "\nChunk cull time:  {:.2}ms - Avg chunks culled: {:.3}",
                    cull_time_avg * 1000.,
                    culled_chunks_avg
                );
            }
            
        }

        let mut chunk_updates_per_second_avg = 0.;
        let mut total_sand_update_second_avg = 0.;
        for (time, count) in &stats.sand_update_time {
            chunk_updates_per_second_avg += *count as f64 / time;
            total_sand_update_second_avg += time;
        }
        chunk_updates_per_second_avg =
            chunk_updates_per_second_avg / (stats.sand_update_time.len() as f64);
        total_sand_update_second_avg =
            total_sand_update_second_avg / (stats.sand_update_time.len() as f64);

        text.sections[5].value = format!(
            "\nSand update time: {:.2}ms - Avg time per chunk: {:.3}ms",
            total_sand_update_second_avg * 1000.,
            1000. / chunk_updates_per_second_avg
        );
    } else {
        *vis = Visibility::Hidden;
    }
}

#[derive(Component)]
struct ToolSelector {
    brush_mode: BrushMode,
    radius: i32,
}

fn spawn_tool_selector_button(
    parent: &mut ChildBuilder,
    asset_server: &Res<AssetServer>,
    label: &str,
    brush_mode: BrushMode,
    radius: i32,
) {
    parent
        .spawn(ButtonBundle {
            style: Style {
                width: Val::Px(100.0),
                height: Val::Px(40.),
                margin: UiRect {
                    left: Val::Px(16.0),
                    bottom: Val::Px(16.0),
                    ..default()
                },
                // horizontally center child text
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                align_self: AlignSelf::FlexEnd,
                flex_direction: FlexDirection::Row,
                ..default()
            },
            background_color: NORMAL_BUTTON.into(),
            ..default()
        })
        .insert(ToolSelector { brush_mode, radius })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                label,
                TextStyle {
                    font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                    font_size: 30.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                },
            ));
        });
}

fn setup_buttons(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            margin: UiRect::horizontal(Val::Px(25.)),
            align_self: AlignSelf::End,
            ..Default::default()
        },
        ..Default::default()
    }).with_children(|parent| {
        spawn_tool_selector_button(parent, &asset_server, "BOMB", BrushMode::Ball, 10);
        spawn_tool_selector_button(parent, &asset_server, "BEAM", BrushMode::Beam, 10);
        spawn_tool_selector_button(parent, &asset_server, "MELT", BrushMode::Melt, 10);
        spawn_tool_selector_button(parent, &asset_server, "BREAK", BrushMode::Break, 10);
        spawn_tool_selector_button(parent, &asset_server, "CHILL", BrushMode::Chill, 20);
        spawn_tool_selector_button(
             parent,
            &asset_server,
            "Stone",
            BrushMode::Place(ParticleType::Stone, 0),
            20,
        );
        spawn_tool_selector_button(
            parent,
            &asset_server,
            "Gravel",
            BrushMode::Place(ParticleType::Gravel, 0),
            10,
        );
        spawn_tool_selector_button(
            parent,
            &asset_server,
            "Sand",
            BrushMode::Place(ParticleType::Sand, 0),
            10,
        );
        spawn_tool_selector_button(
            parent,
            &asset_server,
            "Ice",
            BrushMode::Place(ParticleType::Ice, 0),
            10,
        );
        spawn_tool_selector_button(
            parent,
            &asset_server,
            "Water",
            BrushMode::Place(ParticleType::Water, 0),
            10,
        );
        spawn_tool_selector_button(
            parent,
            &asset_server,
            "Steam",
            BrushMode::Place(ParticleType::Steam, 0),
            10,
        );
        spawn_tool_selector_button(
            parent,
            &asset_server,
            "Lava",
            BrushMode::Place(ParticleType::Lava, 0),
            10,
        );
        spawn_tool_selector_button(
            parent,
            &asset_server,
            "Emit",
            BrushMode::Place(ParticleType::Source, 0),
            1,
        );
        spawn_tool_selector_button(
            parent,
            &asset_server,
            "LaserR",
            BrushMode::Place(ParticleType::LaserEmitter, 1),
            1,
        );
        spawn_tool_selector_button(
            parent,
            &asset_server,
            "LaserL",
            BrushMode::Place(ParticleType::LaserEmitter, 3),
            1,
        );
        spawn_tool_selector_button(
            parent,
            &asset_server,
            "LaserU",
            BrushMode::Place(ParticleType::LaserEmitter, 0),
            1,
        );
        spawn_tool_selector_button(
            parent,
            &asset_server,
            "LaserD",
            BrushMode::Place(ParticleType::LaserEmitter, 2),
            1,
        );
    });
    
}

fn button_system(
    mut capture_state: ResMut<PointerCaptureState>,
    mut interaction_query: Query<(&Interaction, &mut BackgroundColor, &ToolSelector), With<Button>>,
    mut brush_options: ResMut<BrushOptions>,
) {
    capture_state.click_consumed = false;

    for (interaction, mut color, selector) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                brush_options.brush_mode = selector.brush_mode.clone();
                brush_options.radius = selector.radius;
                capture_state.click_consumed = true;
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
            }
        }

        if selector.brush_mode == brush_options.brush_mode {
            *color = PRESSED_BUTTON.into();
        }
    }
}
