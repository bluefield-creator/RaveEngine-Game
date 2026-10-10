use crate::common::core::vrtx::{
    VrtxBrick, VrtxFileState, VrtxImage, VrtxLighting, VrtxMesh, VrtxPlayers, VrtxScript,
    VrtxSettings, VrtxSound, VrtxTexture,
};
use crate::common::game::assets::components::{Image, ImageFace, Mesh, Sound, Texture};
use crate::common::game::bricks::components::{Brick, BrickPhysics, BrickShapeComponent};
use crate::common::game::bricks::studs::{ShadowOpacityExtension, StudsExtension};
use crate::scripting::ecs::{LocalScript, ModuleScript, ServerScript};
use crate::studio::tools::{PlayersService, Selection};
use crate::studio::ui::resources::{InEditorPlaytestClient, PlaytestBackup};
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub type BrickMaterials = Assets<ExtendedMaterial<StandardMaterial, ShadowOpacityExtension>>;
pub type StudsMaterials = Assets<ExtendedMaterial<StandardMaterial, StudsExtension>>;

pub type EntitiesQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Transform,
        &'static Name,
        Option<&'static ChildOf>,
        Option<&'static Children>,
        Option<&'static Brick>,
        Option<&'static mut BrickShapeComponent>,
        &'static GlobalTransform,
        Option<&'static Mesh3d>,
        Option<&'static MeshMaterial3d<ExtendedMaterial<StandardMaterial, ShadowOpacityExtension>>>,
        Option<&'static MeshMaterial3d<ExtendedMaterial<StandardMaterial, StudsExtension>>>,
        Option<&'static mut BrickPhysics>,
    ),
    Without<Camera3d>,
>;

pub type ExplorerQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Name,
        Option<&'static ChildOf>,
        Option<&'static Children>,
        Option<&'static Brick>,
        Option<&'static ServerScript>,
        Option<&'static LocalScript>,
        Option<&'static ModuleScript>,
        Option<&'static Image>,
        Option<&'static Texture>,
        Option<&'static Mesh>,
        Option<&'static Sound>,
    ),
    Without<Camera3d>,
>;

pub type ImagesQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Name,
        Option<&'static ChildOf>,
        Option<&'static Image>,
    ),
    Without<Camera3d>,
>;

pub type ReplicatedImagesQuery<'w, 's> =
    Query<'w, 's, Entity, (With<Image>, With<lightyear::prelude::Replicate>)>;

pub type PlaytestClientQuery<'w, 's> = Query<'w, 's, Entity, With<InEditorPlaytestClient>>;

const PLAYTEST_MAP_PATH: &str = "temp_play.vrtx";
const PLAYTEST_UKEY: &str = "studio_play_local_key";
const DEFAULT_BRICK_COLOR: Srgba = Srgba::new(0.84, 0.24, 0.16, 1.0);

type PhysicsFields = (bool, f32, bool, f32, f32, f32);

fn physics_fields(phys: Option<&BrickPhysics>) -> PhysicsFields {
    phys.map_or((true, 0.3, true, 0.3, 1.0, 1.0), |phys| {
        (
            phys.enabled,
            phys.bounciness,
            phys.player_can_collide,
            phys.friction,
            phys.gravity_scale,
            phys.mass,
        )
    })
}

fn clear_selection(selection: &mut Selection) {
    selection.entity = None;
    selection.entities.clear();
    selection.workspace_selected = false;
    selection.players_selected = false;
    selection.lighting_selected = false;
}

fn explorer_parent_name(explorer_query: &ExplorerQuery, child_of: Option<&ChildOf>) -> Option<String> {
    explorer_query
        .get(child_of?.parent())
        .ok()
        .map(|(_, name, ..)| name.to_string())
}

fn entity_transform(entities_query: &EntitiesQuery, entity: Entity) -> Transform {
    entities_query
        .get(entity)
        .map(|(_, transform, ..)| *transform)
        .unwrap_or_default()
}

fn attach_to_named_parent(
    commands: &mut Commands,
    named_entities: &HashMap<String, Entity>,
    parent_name: Option<&String>,
    child: Entity,
) {
    if let Some(&parent) = parent_name.and_then(|name| named_entities.get(name)) {
        commands.entity(parent).add_child(child);
    }
}

pub fn start_playtest(
    commands: &mut Commands,
    playtest_state: &mut crate::client::PlaytestState,
    playtest_backup: &mut PlaytestBackup,
    selection: &mut Selection,
    gravity: &Option<ResMut<avian3d::prelude::Gravity>>,
    players_service: &Option<ResMut<PlayersService>>,
    entities_query: &EntitiesQuery,
    explorer_query: &ExplorerQuery,
    images_query: &ImagesQuery,
    studs_query: &Query<&crate::common::game::bricks::components::BrickStuds>,
    brick_colors: &Query<&mut crate::common::game::bricks::components::BrickColor>,
    mesh_assets: &Query<&Mesh>,
    sound_assets: &Query<&Sound>,
    graphics_settings: &crate::studio::ui::GraphicsSettings,
    lighting_config: &crate::client::sky::LightingConfig,
    materials: &BrickMaterials,
    studs_materials: &StudsMaterials,
) {
    playtest_state.active = true;
    clear_selection(selection);
    crate::scripting::output::start_run("Playtest");

    playtest_backup.gravity = gravity.as_ref().map(|g| g.0);
    playtest_backup.players_service = players_service.as_ref().map(|ps| (**ps).clone());

    backup_scene(
        commands,
        playtest_backup,
        entities_query,
        explorer_query,
        images_query,
        studs_query,
        brick_colors,
        mesh_assets,
        sound_assets,
    );

    let players = players_service
        .as_ref()
        .map(|ps| VrtxPlayers {
            speed: ps.speed,
            jump_power: ps.jump_power,
            gravity: ps.gravity,
            speed_response: ps.speed_response,
            friction: ps.friction,
            bounciness: ps.bounciness,
        })
        .unwrap_or_default();
    let state = VrtxFileState {
        version: crate::common::core::vrtx::FORMAT_VERSION,
        gravity: Vec3::new(0.0, -186.9 * 0.28, 0.0),
        settings: VrtxSettings {
            ssao: graphics_settings.ssao,
            contact_shadows: graphics_settings.contact_shadows,
            bloom: graphics_settings.bloom,
        },
        lighting: VrtxLighting::from(lighting_config),
        players,
        camera_transform: Transform::IDENTITY,
        bricks: playtest_backup
            .bricks
            .iter()
            .map(|b| {
                let material_color = match (&b.studs_material, &b.standard_material) {
                    (Some(handle), _) => studs_materials.get(&handle.0).map(|m| m.base.base_color),
                    (None, Some(handle)) => materials.get(&handle.0).map(|m| m.base.base_color),
                    (None, None) => None,
                };
                let (physics_enabled, bounciness, player_can_collide, friction, gravity_scale, mass) =
                    physics_fields(b.physics.as_ref());
                VrtxBrick {
                    name: b.name.clone(),
                    transform: b.transform,
                    shape: b.shape,
                    color: material_color.unwrap_or(Color::Srgba(DEFAULT_BRICK_COLOR)),
                    physics_enabled,
                    bounciness,
                    player_can_collide,
                    friction,
                    gravity_scale,
                    mass,
                    show_studs: b.studs,
                }
            })
            .collect(),
        scripts: playtest_backup.scripts.clone(),
        images: playtest_backup.images.clone(),
        meshes: playtest_backup.meshes.clone(),
        textures: playtest_backup.textures.clone(),
        sounds: playtest_backup.sounds.clone(),
    };

    if state.save_to_file(PLAYTEST_MAP_PATH).is_err() {
        return;
    }

    crate::app::server::bootstrap::begin_playtest_server_epoch();
    let playtest_port = crate::app::server::bootstrap::pick_free_playtest_port();
    let playtest_key: [u8; 32] = rand::random();
    let playtest_protocol_id: u64 = rand::random();

    let server_app = crate::app::server::bootstrap::RaveServerApp::new(
        crate::app::server::config::ServerAppConfig {
            port: playtest_port,
            map_path: PLAYTEST_MAP_PATH.to_string(),
            bind_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
            netcode_key: playtest_key,
            protocol_id: playtest_protocol_id,
            allow_unauthenticated: true,
        },
    );
    std::thread::spawn(move || {
        server_app.run();
    });
    std::thread::sleep(std::time::Duration::from_millis(100));

    commands.insert_resource(crate::client::ClientUkey(PLAYTEST_UKEY.to_string()));
    let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), playtest_port);
    if let Some(entity) = crate::app::client::network_boot::spawn_game_client(
        commands,
        server_addr,
        playtest_key,
        playtest_protocol_id,
        InEditorPlaytestClient,
    ) {
        commands.trigger(lightyear::prelude::client::Connect { entity });
    }
}

fn backup_scene(
    commands: &mut Commands,
    playtest_backup: &mut PlaytestBackup,
    entities_query: &EntitiesQuery,
    explorer_query: &ExplorerQuery,
    images_query: &ImagesQuery,
    studs_query: &Query<&crate::common::game::bricks::components::BrickStuds>,
    brick_colors: &Query<&mut crate::common::game::bricks::components::BrickColor>,
    mesh_assets: &Query<&Mesh>,
    sound_assets: &Query<&Sound>,
) {
    playtest_backup.bricks.clear();
    for (entity, _, _, _, _, brick_opt, ..) in entities_query.iter() {
        if brick_opt.is_none() {
            continue;
        }
        if let Some(data) = crate::common::game::bricks::data::capture_brick_data(
            entity,
            entities_query,
            studs_query,
            brick_colors,
            mesh_assets,
            sound_assets,
        ) {
            playtest_backup.bricks.push(data);
        }
        commands.entity(entity).despawn();
    }

    playtest_backup.scripts.clear();
    for (entity, name, child_of, _, _, server, local, module, ..) in explorer_query.iter() {
        let (script_type, code, enabled) = if let Some(s) = server {
            (0, s.code.clone(), s.enabled)
        } else if let Some(l) = local {
            (1, l.code.clone(), l.enabled)
        } else if let Some(m) = module {
            (2, m.code.clone(), true)
        } else {
            continue;
        };
        playtest_backup.scripts.push(VrtxScript {
            name: name.to_string(),
            script_type,
            code,
            parent_name: explorer_parent_name(explorer_query, child_of),
            enabled,
        });
        commands.entity(entity).despawn();
    }

    playtest_backup.images.clear();
    for (entity, name, child_of, image) in images_query.iter() {
        let Some(image) = image else {
            continue;
        };
        let parent_name = child_of
            .and_then(|child_of| images_query.get(child_of.parent()).ok())
            .map(|(_, parent_name, ..)| parent_name.to_string());
        playtest_backup.images.push(VrtxImage {
            name: name.to_string(),
            asset_id: image.asset_id,
            face: image.face.as_ref().map(|f| f.as_str().to_string()),
            parent_name,
            transform: entity_transform(entities_query, entity),
        });
        commands.entity(entity).despawn();
    }

    playtest_backup.meshes.clear();
    for (entity, name, child_of, .., mesh, _) in explorer_query.iter() {
        let Some(mesh) = mesh else {
            continue;
        };
        let (physics_enabled, bounciness, player_can_collide, friction, gravity_scale, mass) =
            entities_query
                .get(entity)
                .map(|(.., phys)| physics_fields(phys))
                .unwrap_or_else(|_| physics_fields(None));
        playtest_backup.meshes.push(VrtxMesh {
            name: name.to_string(),
            asset_id: mesh.asset_id,
            normalize: mesh.normalize,
            parent_name: explorer_parent_name(explorer_query, child_of),
            transform: entity_transform(entities_query, entity),
            physics_enabled,
            bounciness,
            player_can_collide,
            friction,
            gravity_scale,
            mass,
        });
        commands.entity(entity).despawn();
    }

    playtest_backup.textures.clear();
    for (entity, name, child_of, .., texture, _, _) in explorer_query.iter() {
        let Some(texture) = texture else {
            continue;
        };
        playtest_backup.textures.push(VrtxTexture {
            name: name.to_string(),
            id_string: texture.as_content_id(),
            parent_name: explorer_parent_name(explorer_query, child_of),
        });
        commands.entity(entity).despawn();
    }

    playtest_backup.sounds.clear();
    for (entity, name, child_of, .., sound) in explorer_query.iter() {
        let Some(sound) = sound else {
            continue;
        };
        playtest_backup.sounds.push(VrtxSound {
            name: name.to_string(),
            asset_id: sound.asset_id,
            volume: sound.volume,
            speed: sound.speed,
            looped: sound.looped,
            replicate_time: sound.replicate_time,
            playing: sound.playing,
            spatial: sound.spatial,
            parent_name: explorer_parent_name(explorer_query, child_of),
            transform: entity_transform(entities_query, entity),
        });
        commands.entity(entity).despawn();
    }
}

pub fn stop_playtest(
    commands: &mut Commands,
    playtest_state: &mut crate::client::PlaytestState,
    playtest_backup: &mut PlaytestBackup,
    selection: &mut Selection,
    gravity: &mut Option<ResMut<avian3d::prelude::Gravity>>,
    players_service: &mut Option<ResMut<PlayersService>>,
    entities_query: &EntitiesQuery,
    explorer_query: &ExplorerQuery,
    replicated_images: &ReplicatedImagesQuery,
    playtest_client_query: &PlaytestClientQuery,
) {
    playtest_state.active = false;
    clear_selection(selection);

    crate::scripting::output::end_run();
    crate::app::server::bootstrap::request_playtest_server_shutdown();

    if let Some(client_entity) = playtest_client_query.iter().next() {
        commands.trigger(lightyear::prelude::client::Disconnect {
            entity: client_entity,
        });
        commands.entity(client_entity).try_despawn();
    }

    for (entity, _, name, _, _, brick_opt, ..) in entities_query.iter() {
        let name = name.as_str();
        if brick_opt.is_some()
            || name == "Player"
            || name == "LocalPlayer"
            || name.starts_with("Player_")
        {
            commands.entity(entity).try_despawn();
        }
    }
    for entity in replicated_images {
        commands.entity(entity).try_despawn();
    }
    for (entity, .., mesh_opt, sound_opt) in explorer_query.iter() {
        if mesh_opt.is_some() || sound_opt.is_some() {
            commands.entity(entity).try_despawn();
        }
    }

    restore_playtest_backup(
        commands,
        playtest_backup,
        entities_query,
        gravity,
        players_service,
    );
}

pub fn restore_playtest_backup(
    commands: &mut Commands,
    playtest_backup: &mut PlaytestBackup,
    entities_query: &EntitiesQuery,
    gravity: &mut Option<ResMut<avian3d::prelude::Gravity>>,
    players_service: &mut Option<ResMut<PlayersService>>,
) {
    restore_scene(commands, playtest_backup);

    for (entity, ..) in entities_query.iter() {
        commands.entity(entity).remove::<(
            avian3d::prelude::RigidBody,
            avian3d::prelude::Collider,
            crate::common::game::physics::TransformBackup,
            crate::common::game::physics::PhysicsAttached,
            avian3d::prelude::Friction,
            avian3d::prelude::Restitution,
            avian3d::prelude::GravityScale,
            avian3d::prelude::Mass,
            avian3d::prelude::LinearDamping,
            avian3d::prelude::AngularDamping,
            avian3d::prelude::SleepThreshold,
        )>();
    }

    if let (Some(gravity_val), Some(g)) = (playtest_backup.gravity.take(), gravity.as_mut()) {
        g.0 = gravity_val;
    }
    if let Some(ps_val) = playtest_backup.players_service.take() {
        if let Some(ps) = players_service {
            **ps = ps_val.clone();
        }
        if let Ok(mut shared) = crate::studio::tools::SHARED_PLAYERS_SERVICE.write() {
            *shared = ps_val;
        }
    }
}

fn restore_scene(commands: &mut Commands, playtest_backup: &mut PlaytestBackup) {
    let mut named_entities = HashMap::new();
    for brick_data in playtest_backup.bricks.drain(..) {
        let entity = crate::common::game::bricks::data::spawn_from_data(commands, &brick_data);
        named_entities.insert(brick_data.name, entity);
    }

    for script_data in playtest_backup.scripts.drain(..) {
        let mut cmd = commands.spawn(Name::new(script_data.name));
        match script_data.script_type {
            0 => {
                cmd.insert(ServerScript {
                    code: script_data.code,
                    enabled: script_data.enabled,
                    ..default()
                });
            }
            1 => {
                cmd.insert((
                    LocalScript {
                        code: script_data.code,
                        enabled: script_data.enabled,
                        ..default()
                    },
                    lightyear::prelude::Replicate::default(),
                ));
            }
            _ => {
                cmd.insert((
                    ModuleScript {
                        code: script_data.code,
                    },
                    lightyear::prelude::Replicate::default(),
                ));
            }
        }
        let entity = cmd.id();
        attach_to_named_parent(
            commands,
            &named_entities,
            script_data.parent_name.as_ref(),
            entity,
        );
    }

    for image_data in playtest_backup.images.drain(..) {
        let face = image_data.face.as_deref().and_then(ImageFace::from_str);
        let entity = commands
            .spawn((
                image_data.transform,
                Name::new(image_data.name),
                Image {
                    asset_id: image_data.asset_id,
                    face,
                },
                Pickable::default(),
                Visibility::Visible,
            ))
            .id();
        attach_to_named_parent(
            commands,
            &named_entities,
            image_data.parent_name.as_ref(),
            entity,
        );
    }

    let mut restored_meshes = Vec::new();
    for mesh_data in playtest_backup.meshes.drain(..) {
        let entity = commands
            .spawn((
                mesh_data.transform,
                Name::new(mesh_data.name.clone()),
                Mesh {
                    asset_id: mesh_data.asset_id,
                    normalize: mesh_data.normalize,
                },
                BrickPhysics {
                    enabled: mesh_data.physics_enabled,
                    bounciness: mesh_data.bounciness,
                    player_can_collide: mesh_data.player_can_collide,
                    friction: mesh_data.friction,
                    gravity_scale: mesh_data.gravity_scale,
                    mass: mesh_data.mass,
                },
                avian3d::prelude::CollisionLayers::from_bits(0b0001, 0xFFFF_FFFF),
                Pickable::default(),
                Visibility::Visible,
            ))
            .id();
        named_entities.insert(mesh_data.name, entity);
        restored_meshes.push((entity, mesh_data.asset_id));
        attach_to_named_parent(
            commands,
            &named_entities,
            mesh_data.parent_name.as_ref(),
            entity,
        );
    }

    let mut textured_meshes = std::collections::HashSet::new();
    for texture_data in playtest_backup.textures.drain(..) {
        let parsed = Texture::parse_content_id(&texture_data.id_string);
        let entity = commands
            .spawn((
                Name::new(texture_data.name),
                Texture {
                    asset_id: parsed.map(|(id, _)| id).unwrap_or(0),
                    is_decal: parsed.map(|(_, decal)| decal).unwrap_or(false),
                },
            ))
            .id();
        if let Some(&parent) = texture_data
            .parent_name
            .as_ref()
            .and_then(|name| named_entities.get(name))
        {
            commands.entity(parent).add_child(entity);
            textured_meshes.insert(parent);
        }
    }
    for (mesh_entity, asset_id) in restored_meshes {
        if textured_meshes.contains(&mesh_entity) {
            continue;
        }
        let texture_entity = commands
            .spawn((
                Name::new("Texture"),
                Texture {
                    asset_id,
                    is_decal: false,
                },
            ))
            .id();
        commands.entity(mesh_entity).add_child(texture_entity);
    }

    for sound_data in playtest_backup.sounds.drain(..) {
        let entity = commands
            .spawn((
                sound_data.transform,
                Name::new(sound_data.name.clone()),
                Sound {
                    asset_id: sound_data.asset_id,
                    volume: Sound::clamp_volume(sound_data.volume),
                    speed: Sound::clamp_speed(sound_data.speed),
                    looped: sound_data.looped,
                    replicate_time: sound_data.replicate_time,
                    playing: sound_data.playing,
                    spatial: sound_data.spatial,
                    position: 0.0,
                },
                Pickable::default(),
                Visibility::Visible,
            ))
            .id();
        named_entities.insert(sound_data.name, entity);
        attach_to_named_parent(
            commands,
            &named_entities,
            sound_data.parent_name.as_ref(),
            entity,
        );
    }
}
