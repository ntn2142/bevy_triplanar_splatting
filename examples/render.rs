use bevy::{
    asset::RenderAssetUsages,
    image::{
        CompressedImageFormats, ImageAddressMode, ImageFilterMode, ImageLoader, ImageSampler,
        ImageSamplerDescriptor,
    },
    log,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension},
        renderer::RenderDevice,
    },
};
use bevy_triplanar_splatting::{
    TriplanarMaterialPlugin,
    triplanar_material::{ATTRIBUTE_MATERIAL_WEIGHTS, TriplanarMaterial},
};
use smooth_bevy_cameras::{LookTransformPlugin, controllers::fps::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TriplanarMaterialPlugin)
        // .add_plugin(WireframePlugin::default())
        .add_plugins(LookTransformPlugin)
        .add_plugins(FpsCameraPlugin::default())
        // .register_asset_loader(ImageLoader::new(CompressedImageFormats::all()))
        .add_systems(Startup, setup)
        .add_systems(Update, (spawn_meshes, move_lights))
        .run();
}

/// set up a simple 3D scene
fn setup(
    asset_server: Res<AssetServer>,
    device: Res<RenderDevice>,
    mut commands: Commands,
    // mut wireframe_config: ResMut<WireframeConfig>,
) {
    // wireframe_config.global = true;

    // List all available device features so we can tell what texture formats
    // are supported.
    println!("DEVICE FEATURES = {:?}", device.features());

    // start loading materials
    // TODO: automatically choose textures based on GPU supported features

    // TODO: load some textures and run
    // let t = LoadingImageArray::<4>::new([]);
    commands.insert_resource(MaterialHandles {
        base_color: LoadingImageArray::new([], &asset_server),
        occlusion: LoadingImageArray::new([], &asset_server),
        normal_map: LoadingImageArray::new([], &asset_server),
        metal_rough: LoadingImageArray::new([], &asset_server),
        spawned: false,
    });

    // commands.insert_resource(MaterialHandles {
    //     base_color: LoadingImage::new(asset_server.load("array_material/albedo.ktx2")),
    //     occlusion: LoadingImage::new(asset_server.load("array_material/ao.ktx2")),
    //     normal_map: LoadingImage::new(asset_server.load("array_material/normal.ktx2")),
    //     metal_rough: LoadingImage::new(asset_server.load("array_material/metal_rough.ktx2")),
    //     spawned: false,
    // });

    // commands.insert_resource(MaterialHandles {
    //     base_color: LoadingImage::new(asset_server.load("array_material/albedo.basis")),
    //     occlusion: LoadingImage::new(asset_server.load("array_material/ao.basis")),
    //     normal_map: LoadingImage::new(asset_server.load("array_material/normal.basis")),
    //     metal_rough: LoadingImage::new(asset_server.load("array_material/metal_rough.basis")),
    //     spawned: false,
    // });

    // commands.insert_resource(AmbientLight {
    //     brightness: 2.0,
    //     ..default()
    // });

    // Spawn lights and camera.
    commands.spawn((
        MovingLight,
        PointLight {
            intensity: 50000.,
            range: 100.,
            ..default()
        },
    ));

    commands
        .spawn(Camera3d::default())
        .insert(FpsCameraBundle::new(
            FpsCameraController {
                translate_sensitivity: 8.0,
                ..Default::default()
            },
            Vec3::new(12.0, 12.0, 12.0),
            Vec3::new(0., 0., 0.),
            Vec3::Y,
        ));
}

#[derive(Component)]
struct MovingLight;

fn move_lights(time: Res<Time>, mut lights: Query<(&MovingLight, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (_, mut tfm) in lights.iter_mut() {
        tfm.translation = 15.0 * Vec3::new(t.cos(), 1.0, t.sin());
    }
}

struct LoadingImageArray<const n: usize> {
    images: [LoadingImage; n],
}

impl<const N: usize> LoadingImageArray<N> {
    fn new(paths: [&str; N], asset_server: &AssetServer) -> Self {
        let arr = paths.map(|path| LoadingImage::new(asset_server.load(path)));
        Self { images: arr }
    }
    fn check_any_loaded(&mut self, created_handle: &AssetId<Image>) -> bool {
        self.images
            .iter_mut()
            .any(|im| im.check_loaded(created_handle))
    }
    fn all_loaded(&self) -> bool {
        self.images.iter().all(|im| im.loaded)
    }

    fn merge_images(self, assets: &mut Assets<Image>) -> Handle<Image> {
        let images = self.images.map(|im| assets.get(im.handle.id()).unwrap());
        let mut size = images[0].texture_descriptor.size;
        let format = images[0].texture_descriptor.format;
        let usage = images[0].asset_usage;
        if N >= 2 {
            let all_match = images[1..].iter().all(|im| {
                let s = im.texture_descriptor.size;
                let f = im.texture_descriptor.format;
                s.width == size.width
                    && s.height == size.height
                    && f == format
                    && im.asset_usage == usage
            });
            if !all_match {
                log::error!("size or format dont match");
                panic!()
            }
        }

        let mut new_data = Vec::with_capacity(
            images
                .iter()
                .map(|im| im.data.as_ref().unwrap().len())
                .sum(),
        );
        images
            .iter()
            .for_each(|im| new_data.extend_from_slice(im.data.as_ref().unwrap().as_slice()));

        size.height *= N as u32;
        size.depth_or_array_layers = 1;
        let new = Image::new_fill(size, TextureDimension::D2, &new_data, format, usage);
        assets.add(new)
    }
}

#[derive(Resource)]
struct MaterialHandles<const N: usize> {
    base_color: LoadingImageArray<N>,
    occlusion: LoadingImageArray<N>,
    normal_map: LoadingImageArray<N>,
    metal_rough: LoadingImageArray<N>,
    spawned: bool,
}

impl<const N: usize> MaterialHandles<N> {
    fn all_loaded(&self) -> bool {
        self.base_color.all_loaded()
            && self.occlusion.all_loaded()
            && self.normal_map.all_loaded()
            && self.metal_rough.all_loaded()
    }

    fn check_loaded(&mut self, created_handle: &AssetId<Image>) -> bool {
        // Check every handle without short circuiting because they might be
        // duplicates.
        let mut any_loaded = false;
        any_loaded |= self.base_color.check_any_loaded(created_handle);
        any_loaded |= self.occlusion.check_any_loaded(created_handle);
        any_loaded |= self.normal_map.check_any_loaded(created_handle);
        any_loaded |= self.metal_rough.check_any_loaded(created_handle);
        any_loaded
    }
}

struct LoadingImage {
    handle: Handle<Image>,
    loaded: bool,
}

impl LoadingImage {
    fn new(handle: Handle<Image>) -> Self {
        Self {
            handle,
            loaded: false,
        }
    }

    fn check_loaded(&mut self, created_handle: &AssetId<Image>) -> bool {
        if *created_handle == self.handle.id() {
            self.loaded = true;
            true
        } else {
            false
        }
    }
}

fn spawn_meshes(
    mut asset_events: EventReader<AssetEvent<Image>>,
    mut assets: ResMut<Assets<Image>>,
    mut commands: Commands,
    mut handles: ResMut<MaterialHandles>,
    mut materials: ResMut<Assets<TriplanarMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if handles.spawned {
        return;
    }

    for event in asset_events.read() {
        if let &AssetEvent::LoadedWithDependencies { id } = event {
            if !handles.check_loaded(&id) {
                continue;
            }
            // if any of our textures load do this

            let texture = assets.get_mut(id).unwrap();
            texture.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                address_mode_w: ImageAddressMode::Repeat,
                min_filter: ImageFilterMode::Linear,
                mag_filter: ImageFilterMode::Linear,
                mipmap_filter: ImageFilterMode::Linear,
                ..default()
            });
        }
    }

    if !handles.all_loaded() {
        return;
    }
    handles.spawned = true;

    let mut sphere_mesh = Mesh::try_from(Sphere::new(5.0).mesh().ico(6).unwrap()).unwrap();

    let material_weights: Vec<u32> = sphere_mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .unwrap()
        .as_float3()
        .unwrap()
        .iter()
        .map(|p| {
            let p = Vec3::from(*p);
            let w = sigmoid(signed_weight_to_unsigned(p.dot(Vec3::X)), 10.0);
            let w0 = (w * 255.0).clamp(0.0, 255.0) as u32;
            let w1 = 255 - w0;
            encode_weights([w0, 0, w1, 0])
            // encode_weights([255, 0, 0, 0])
        })
        .collect();
    sphere_mesh.insert_attribute(ATTRIBUTE_MATERIAL_WEIGHTS, material_weights);

    commands.spawn((
        Mesh3d(meshes.add(sphere_mesh)),
        MeshMaterial3d(materials.add(TriplanarMaterial {
            metallic: 0.05,
            perceptual_roughness: 0.9,

            base_color_texture: Some(handles.base_color.handle.clone()),
            emissive_texture: None,
            metallic_roughness_texture: Some(handles.metal_rough.handle.clone()),
            normal_map_texture: Some(handles.normal_map.handle.clone()),
            occlusion_texture: Some(handles.occlusion.handle.clone()),

            uv_scale: 1.0,
            ..default()
        })),
    ));
}

/// Linear transformation from domain `[-1.0, 1.0]` into range `[0.0, 1.0]`.
fn signed_weight_to_unsigned(x: f32) -> f32 {
    0.5 * (x + 1.0)
}

fn encode_weights(w: [u32; 4]) -> u32 {
    w[0] | (w[1] << 8) | (w[2] << 16) | (w[3] << 24)
}

fn sigmoid(x: f32, beta: f32) -> f32 {
    1.0 / (1.0 + (x / (1.0 - x)).powf(-beta))
}
