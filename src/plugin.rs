use crate::triplanar_material::TriplanarMaterial;
use bevy::asset::{embedded_asset, load_internal_asset, weak_handle};
use bevy::prelude::*;

const TRIPLANAR_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("0cdc37f0-b08f-42f9-8e80-368e3b79484d");
const BIPLANAR_SHADER_HANDLE: Handle<Shader> = weak_handle!("c4884a47-d77a-45bc-96d1-56c3ff4c0811");

pub struct TriplanarMaterialPlugin;

impl Plugin for TriplanarMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TriplanarMaterial>::default());

        load_internal_asset!(
            app,
            TRIPLANAR_SHADER_HANDLE,
            "shaders/triplanar.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            BIPLANAR_SHADER_HANDLE,
            "shaders/biplanar.wgsl",
            Shader::from_wgsl
        );
        embedded_asset!(app, "shaders/triplanar_material_vert.wgsl");
        embedded_asset!(app, "shaders/triplanar_material_frag.wgsl");
    }
}
