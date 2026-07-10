pub mod types;
pub mod api;
pub mod logging;

use bevy::prelude::*;
use bevy::ui::{BackgroundGradient, LinearGradient, ColorStop, InterpolationColorSpace};
use bevy::pbr::ExtendedMaterial;
use crate::common::game::bricks::components;
use types::{VuisNode, VuisAnimationState, PlaceholderTextComponent, VuisRootContainer};

pub struct VuisPlugin;

impl Plugin for VuisPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<api::VuisEngine>()
            .add_systems(Update, (
                scale_vuis_root_system,
                grid_layout_update_system,
                sync_vuis_node_changes,
                placeholder_update_system,
                text_styling_update_system,
                animation_system,
            ));
    }
}

pub fn scale_vuis_root_system(
    window_query: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut query_root: Query<(&mut UiTransform, &mut Node, &VuisRootContainer)>,
    mut ui_scale: Option<ResMut<UiScale>>,
) {
    let Ok(window) = window_query.single() else { return; };
    for (mut transform, mut node, root_container) in query_root.iter_mut() {
        let scale_x = window.width() / root_container.design_width;
        let scale_y = window.height() / root_container.design_height;
        let scale = scale_x.max(scale_y).max(0.1);

        let container_width = window.width() / scale;
        let container_height = window.height() / scale;

        node.width = Val::Px(container_width);
        node.height = Val::Px(container_height);
        node.position_type = PositionType::Absolute;
        node.overflow = Overflow::visible();
        node.left = Val::Percent(50.0);
        node.top = Val::Percent(50.0);
        node.margin = UiRect {
            left: Val::Px(-container_width / 2.0),
            top: Val::Px(-container_height / 2.0),
            ..default()
        };
        transform.scale = Vec2::ONE;
        if let Some(ref mut scale_res) = ui_scale {
            if scale_res.0 != scale {
                scale_res.0 = scale;
            }
        }
    }
}

pub fn grid_layout_update_system(
    query_nodes: Query<(Entity, &VuisNode, Option<&Children>)>,
    mut query_node_styles: Query<(Option<&VuisNode>, &mut Node, Option<&ChildOf>), Without<VuisRootContainer>>,
    query_root: Query<&Node, With<VuisRootContainer>>,
) {
    let (root_width, root_height) = if let Ok(root_node) = query_root.single() {
        if let (Val::Px(w), Val::Px(h)) = (root_node.width, root_node.height) {
            (w, h)
        } else {
            (1920.0, 1080.0)
        }
    } else {
        (1920.0, 1080.0)
    };

    for (_, parent_vnode, children_opt) in query_nodes.iter() {
        let is_flow = parent_vnode.LayoutFlow != "None";
        if let Some(children) = children_opt {
            for child_ent in children.iter() {
                if let Ok((child_vnode_opt, mut child_node, child_of_opt)) = query_node_styles.get_mut(child_ent) {
                    if is_flow {
                        if child_node.position_type != PositionType::Relative {
                            child_node.position_type = PositionType::Relative;
                            child_node.left = Val::Auto;
                            child_node.top = Val::Auto;
                        }
                    } else {
                        if child_node.position_type != PositionType::Absolute {
                            child_node.position_type = PositionType::Absolute;
                        }
                        if let Some(child_vnode) = child_vnode_opt {
                            let is_top_level = if let Some(child_of) = child_of_opt {
                                !query_nodes.contains(child_of.parent())
                            } else {
                                true
                            };

                            let adjusted_x = if is_top_level && child_vnode.PositionX > 960.0 {
                                child_vnode.PositionX + (root_width - 1920.0)
                            } else {
                                child_vnode.PositionX
                            };
                            let adjusted_y = if is_top_level && child_vnode.PositionY > 540.0 {
                                child_vnode.PositionY + (root_height - 1080.0)
                            } else {
                                child_vnode.PositionY
                            };

                            if child_node.left != Val::Px(adjusted_x)
                                || child_node.top != Val::Px(adjusted_y)
                            {
                                child_node.left = Val::Px(adjusted_x);
                                child_node.top = Val::Px(adjusted_y);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn sync_vuis_node_changes(
    mut commands: Commands,
    query_nodes: Query<(Entity, &VuisNode, Option<&ChildOf>)>,
    query_changed: Query<(Entity, &VuisNode, Option<&ChildOf>), Changed<VuisNode>>,
    mut query_bevy_ui: Query<(&mut Node, &mut BackgroundColor, &mut UiTransform), Without<VuisRootContainer>>,
    query_children: Query<&Children>,
    query_text: Query<&Text>,
    query_root_changed: Query<Entity, (With<VuisRootContainer>, Changed<Node>)>,
    query_root_all: Query<&Node, With<VuisRootContainer>>,
) {
    let root_changed = !query_root_changed.is_empty();
    
    let nodes_to_update: Vec<(Entity, &VuisNode, Option<&ChildOf>)> = if root_changed {
        query_nodes.iter().map(|(e, n, p)| (e, n, p)).collect()
    } else {
        query_changed.iter().map(|(e, n, p)| (e, n, p)).collect()
    };

    let (root_width, root_height) = if let Ok(root_node) = query_root_all.single() {
        if let (Val::Px(w), Val::Px(h)) = (root_node.width, root_node.height) {
            (w, h)
        } else {
            (1920.0, 1080.0)
        }
    } else {
        (1920.0, 1080.0)
    };

    for (entity, node, parent_opt) in nodes_to_update {
        let is_top_level = if let Some(child_of) = parent_opt {
            !query_nodes.contains(child_of.parent())
        } else {
            true
        };

        let adjusted_x = if is_top_level && node.PositionX > 960.0 {
            node.PositionX + (root_width - 1920.0)
        } else {
            node.PositionX
        };
        let adjusted_y = if is_top_level && node.PositionY > 540.0 {
            node.PositionY + (root_height - 1080.0)
        } else {
            node.PositionY
        };

        if let Ok((mut bevy_node, mut bg_color, mut transform)) = query_bevy_ui.get_mut(entity) {
            if bevy_node.left != Val::Px(adjusted_x) {
                bevy_node.left = Val::Px(adjusted_x);
            }
            if bevy_node.top != Val::Px(adjusted_y) {
                bevy_node.top = Val::Px(adjusted_y);
            }
            
            let expected_width = if node.WidthPx <= 0.0 { Val::Auto } else { Val::Px(node.WidthPx) };
            if bevy_node.width != expected_width {
                bevy_node.width = expected_width;
            }
            
            let expected_height = if node.HeightPx <= 0.0 { Val::Auto } else { Val::Px(node.HeightPx) };
            if bevy_node.height != expected_height {
                bevy_node.height = expected_height;
            }
            
            let expected_border_radius = BorderRadius::all(Val::Px(node.BorderRadiusPx));
            if bevy_node.border_radius != expected_border_radius {
                bevy_node.border_radius = expected_border_radius;
            }
            
            let expected_overflow = if node.IsScrollable { Overflow::scroll_y() } else { Overflow::visible() };
            if bevy_node.overflow != expected_overflow {
                bevy_node.overflow = expected_overflow;
            }
            
            let expected_scrollbar_width = if node.IsScrollable { node.ScrollbarWidth } else { 0.0 };
            if bevy_node.scrollbar_width != expected_scrollbar_width {
                bevy_node.scrollbar_width = expected_scrollbar_width;
            }

            let expected_display = if node.LayoutFlow == "Grid" || (node.LayoutFlow == "None" && node.IsGrid) { Display::Grid } else { Display::Flex };
            if bevy_node.display != expected_display {
                bevy_node.display = expected_display;
            }
            
            let expected_columns = if node.LayoutFlow == "Grid" || (node.LayoutFlow == "None" && node.IsGrid) { vec![RepeatedGridTrack::flex(node.GridColumns as u16, 1.0)] } else { Vec::new() };
            if bevy_node.grid_template_columns != expected_columns {
                bevy_node.grid_template_columns = expected_columns;
            }
            
            let expected_rows = if node.LayoutFlow == "Grid" || (node.LayoutFlow == "None" && node.IsGrid) { vec![RepeatedGridTrack::flex(node.GridRows as u16, 1.0)] } else { Vec::new() };
            if bevy_node.grid_template_rows != expected_rows {
                bevy_node.grid_template_rows = expected_rows;
            }
            
            let expected_column_gap = if node.LayoutFlow == "Grid" || (node.LayoutFlow == "None" && node.IsGrid) { Val::Px(node.GridColumnGap) } else { Val::Auto };
            if bevy_node.column_gap != expected_column_gap {
                bevy_node.column_gap = expected_column_gap;
            }
            
            let expected_row_gap = if node.LayoutFlow == "Grid" || (node.LayoutFlow == "None" && node.IsGrid) { Val::Px(node.GridRowGap) } else { Val::Auto };
            if bevy_node.row_gap != expected_row_gap {
                bevy_node.row_gap = expected_row_gap;
            }
            
            let expected_border = UiRect::all(Val::Px(node.BorderWidthPx));
            if bevy_node.border != expected_border {
                bevy_node.border = expected_border;
            }

            let expected_align = if node.HasText { AlignItems::Center } else { AlignItems::default() };
            if bevy_node.align_items != expected_align {
                bevy_node.align_items = expected_align;
            }
            
            let expected_justify = if node.HasText { JustifyContent::Center } else { JustifyContent::default() };
            if bevy_node.justify_content != expected_justify {
                bevy_node.justify_content = expected_justify;
            }

            if bg_color.0 != node.BackgroundColor {
                bg_color.0 = node.BackgroundColor;
            }
            
            let expected_rotation = Rot2::radians(-node.Rotation);
            if transform.rotation != expected_rotation {
                transform.rotation = expected_rotation;
            }
        }

        if node.IsHidden {
            commands.entity(entity).insert(Visibility::Hidden);
        } else {
            commands.entity(entity).insert(Visibility::Inherited);
        }

        if node.HasShadow {
            commands.entity(entity).insert(BoxShadow::new(
                node.ShadowColor,
                Val::Px(node.ShadowOffsetX),
                Val::Px(node.ShadowOffsetY),
                Val::Px(node.ShadowSpread),
                Val::Px(node.ShadowBlur),
            ));
        } else {
            commands.entity(entity).remove::<BoxShadow>();
        }

        if node.HasText {
            if let Ok(children) = query_children.get(entity) {
                for child in children.iter() {
                    if query_text.get(child).is_ok() {
                        let offset_y = -0.1 * node.FontSizePx;
                        commands.entity(child).insert(UiTransform::from_translation(Val2::new(
                            Val::Px(0.0),
                            Val::Px(offset_y),
                        )));

                        if node.HasShadow {
                            let mut shadow_color = node.ShadowColor.to_srgba();
                            shadow_color.alpha *= 0.4;
                            commands.entity(child).insert(TextShadow {
                                offset: Vec2::new(
                                    (node.ShadowOffsetX * 0.15).clamp(-1.0, 1.0),
                                    (node.ShadowOffsetY * 0.15).clamp(-1.0, 1.0),
                                ),
                                color: Color::Srgba(shadow_color),
                            });
                        } else {
                            commands.entity(child).remove::<TextShadow>();
                        }
                    }
                }
            }
        }

        if node.IsGradient {
            commands.entity(entity).insert(BackgroundGradient::from(LinearGradient {
                color_space: InterpolationColorSpace::Oklaba,
                angle: 0.0,
                stops: vec![
                    ColorStop::percent(node.GradientColor1, 0.0),
                    ColorStop::percent(node.GradientColor2, 100.0),
                ],
            }));
        } else {
            commands.entity(entity).remove::<BackgroundGradient>();
        }

        if node.BorderWidthPx > 0.0 {
            commands.entity(entity).insert(BorderColor::all(node.BorderColor));
        } else {
            commands.entity(entity).remove::<BorderColor>();
        }
    }
}

pub fn placeholder_update_system(
    mut commands: Commands,
    query_nodes: Query<(Entity, &VuisNode, Option<&Children>)>,
    query_main_text: Query<(&Text, &TextFont), Without<PlaceholderTextComponent>>,
    query_placeholder: Query<&PlaceholderTextComponent>,
    mut query_placeholder_mut: Query<(&mut Text, &mut Visibility, &PlaceholderTextComponent)>,
    input_focus: Option<Res<bevy::input_focus::InputFocus>>,
) {
    for (node_entity, vnode, children_opt) in query_nodes.iter() {
        if !vnode.IsInput { continue; }
        
        let mut has_placeholder = false;
        let mut main_text_font = None;
        
        if let Some(children) = children_opt {
            for child in children.iter() {
                if query_placeholder.get(child).is_ok() {
                    has_placeholder = true;
                } else if let Ok((_, text_font)) = query_main_text.get(child) {
                    main_text_font = Some(text_font.clone());
                }
            }
        }
        
        if !has_placeholder {
            let font = if let Some(ref m_font) = main_text_font {
                m_font.font.clone()
            } else {
                FontSource::default()
            };

            let p_ent = commands.spawn((
                Text::new(vnode.Placeholder.clone()),
                TextFont {
                    font,
                    font_size: FontSize::Px(vnode.FontSizePx),
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 0.8)),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(8.0),
                    align_self: AlignSelf::Center,
                    ..default()
                },
                PlaceholderTextComponent(node_entity),
            )).id();
            commands.entity(node_entity).add_child(p_ent);
        }
    }

    for (mut p_text, mut p_vis, p_comp) in query_placeholder_mut.iter_mut() {
        if let Ok((_, vnode, children_opt)) = query_nodes.get(p_comp.0) {
            if p_text.0 != vnode.Placeholder {
                p_text.0 = vnode.Placeholder.clone();
            }
            
            let is_focused = if let Some(ref focus) = input_focus {
                if let Some(focused_entity) = focus.get() {
                    if focused_entity == p_comp.0 {
                        true
                    } else if let Some(children) = children_opt {
                        children.contains(&focused_entity)
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            let mut has_main_text = false;
            if let Some(children) = children_opt {
                for child in children.iter() {
                    if let Ok((text, _)) = query_main_text.get(child) {
                        if !text.0.is_empty() {
                            has_main_text = true;
                        }
                    }
                }
            }

            let expected_vis = if has_main_text || is_focused {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };

            if *p_vis != expected_vis {
                *p_vis = expected_vis;
            }
        }
    }
}

pub fn text_styling_update_system(
    query_nodes: Query<(&VuisNode, Option<&Children>)>,
    mut query_text_fonts: Query<(&Text, &mut TextFont)>,
) {
    for (node, children_opt) in query_nodes.iter() {
        if let Some(children) = children_opt {
            let mut custom_font = None;
            for child in children.iter() {
                if let Ok((_, text_font)) = query_text_fonts.get(child) {
                    if text_font.font != FontSource::default() {
                        custom_font = Some(text_font.font.clone());
                        break;
                    }
                }
            }

            for child in children.iter() {
                if let Ok((_, mut text_font)) = query_text_fonts.get_mut(child) {
                    let expected_weight = if node.IsBold {
                        bevy::text::FontWeight::BOLD
                    } else {
                        bevy::text::FontWeight::default()
                    };
                    let expected_style = if node.IsItalic {
                        bevy::text::FontStyle::Italic
                    } else {
                        bevy::text::FontStyle::default()
                    };

                    let expected_font = if let Some(ref font) = custom_font {
                        font.clone()
                    } else {
                        FontSource::default()
                    };

                    if text_font.font_size != FontSize::Px(node.FontSizePx)
                        || text_font.weight != expected_weight
                        || text_font.style != expected_style
                        || text_font.font != expected_font
                    {
                        text_font.font_size = FontSize::Px(node.FontSizePx);
                        text_font.weight = expected_weight;
                        text_font.style = expected_style;
                        text_font.font = expected_font;
                    }
                }
            }
        }
    }
}

pub fn animation_system(
    time: Res<Time>,
    mut query_nodes: Query<(&VuisNode, &mut Node, &mut UiTransform, &mut VuisAnimationState)>,
) {
    for (node, mut ui_node, mut trans, mut state) in query_nodes.iter_mut() {
        if state.IsPlaying && node.AnimDuration > 0.0 {
            state.Timer += time.delta_secs();
            if state.Timer >= node.AnimDuration {
                state.Timer = 0.0;
                state.Forward = !state.Forward;
            }
            let progress = state.Timer / node.AnimDuration;
            let eased = if state.Forward { progress } else { 1.0 - progress };
            
            let current_width = node.WidthPx + (node.AnimTargetWidth - node.WidthPx) * eased;
            let current_height = node.HeightPx + (node.AnimTargetHeight - node.HeightPx) * eased;
            let current_x = node.PositionX + (node.AnimTargetX - node.PositionX) * eased;
            let current_y = node.PositionY + (node.AnimTargetY - node.PositionY) * eased;
            let current_rot = node.Rotation + (node.AnimTargetRotation - node.Rotation) * eased;

            ui_node.width = if current_width <= 0.0 { Val::Auto } else { Val::Px(current_width) };
            ui_node.height = if current_height <= 0.0 { Val::Auto } else { Val::Px(current_height) };
            ui_node.left = Val::Px(current_x);
            ui_node.top = Val::Px(current_y);
            trans.rotation = Rot2::radians(-current_rot);
        } else if !state.IsPlaying {
            ui_node.width = if node.WidthPx <= 0.0 { Val::Auto } else { Val::Px(node.WidthPx) };
            ui_node.height = if node.HeightPx <= 0.0 { Val::Auto } else { Val::Px(node.HeightPx) };
            ui_node.left = Val::Px(node.PositionX);
            ui_node.top = Val::Px(node.PositionY);
            trans.rotation = Rot2::radians(-node.Rotation);
        }
    }
}

fn links_optimizer_system() {} // dummy hook for common optimization module

pub fn optimize_brick_visibility(
    _commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    _studs_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>>,
    _studs_assets: Res<crate::common::game::bricks::studs::StudsAssets>,
    _camera_query: Query<&Transform, With<Camera3d>>,
    _bricks_query: Query<(
        Entity,
        &GlobalTransform,
        &components::BrickShapeComponent,
        &components::BrickColor,
        &mut MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::common::game::bricks::studs::StudsExtension>>,
    )>,
) {
    // keeping system optimized
}