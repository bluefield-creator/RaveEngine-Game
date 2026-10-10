use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct SkyboxMesh;

pub(crate) fn init_skybox_mesh<M: Material>(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    material: MeshMaterial3d<M>,
) {
    let mesh = meshes.add(Cuboid::new(2.0, 2.0, 2.0));

    commands.spawn((
        Mesh3d(mesh),
        material,
        Transform::default(),
        SkyboxMesh,
        NotShadowCaster,
        NotShadowReceiver,
        NoFrustumCulling,
    ));
}

pub(crate) fn update_skybox_transform(
    camera_query: Query<(&Transform, &Camera, &Projection), (With<Camera3d>, Without<SkyboxMesh>)>,
    mut skybox: Query<(&mut Transform, &mut Visibility), With<SkyboxMesh>>,
) {
    let mut fallback = None;
    for (camera_transform, camera, projection) in &camera_query {
        if !camera.is_active {
            continue;
        }
        let far = match projection {
            Projection::Perspective(pers) => pers.far,
            _ => continue,
        };
        if camera.order == 0 {
            let scale = (far * 0.4).min(5000.0);
            for (mut transform, mut visibility) in skybox.iter_mut() {
                transform.translation = camera_transform.translation;
                transform.scale = Vec3::splat(scale);
                if *visibility != Visibility::Inherited {
                    *visibility = Visibility::Inherited;
                }
            }
            return;
        }
        fallback.get_or_insert((camera_transform.translation, far));
    }
    if let Some((translation, far)) = fallback {
        let scale = (far * 0.4).min(5000.0);
        for (mut transform, mut visibility) in skybox.iter_mut() {
            transform.translation = translation;
            transform.scale = Vec3::splat(scale);
            if *visibility != Visibility::Inherited {
                *visibility = Visibility::Inherited;
            }
        }
    }
}
