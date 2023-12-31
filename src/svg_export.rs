use crate::Resolution;
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::sprite::Mesh2dHandle;
use vsvg::{DocumentTrait, Transforms};

pub struct SvgExportPlugin;

impl Plugin for SvgExportPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SvgExportSettings::default())
            .add_systems(Update, (keyboard_system,))
            .add_systems(Last, (svg_export_system,));
    }
}

#[derive(Resource, Debug, Default)]
pub struct SvgExportSettings {
    pub export_path: String,

    /// Flag to indicate that the export should be run.
    pub run_export: bool,
}

fn keyboard_system(
    mut svg_export_settings: ResMut<SvgExportSettings>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::E) {
        svg_export_settings.run_export = true;
    }
}

fn svg_export_system(
    meshes: Res<Assets<Mesh>>,
    resolution: Res<Resolution>,
    mut svg_export_settings: ResMut<SvgExportSettings>,
    query: Query<(&GlobalTransform, &ComputedVisibility, &Mesh2dHandle)>,
) {
    if svg_export_settings.run_export {
        svg_export_settings.run_export = false;

        let mut doc = vsvg::Document::default();
        doc.metadata_mut().page_size = Some(resolution.as_page_size());

        for (transform, visibility, Mesh2dHandle(mesh_handle)) in query.iter() {
            if !visibility.is_visible() {
                continue;
            }

            let Some(mesh) = meshes.get(mesh_handle) else {
                continue;
            };

            let Some(vertex_attrib) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
                continue;
            };

            let VertexAttributeValues::Float32x3(vertex_data) = &vertex_attrib else {
                continue;
            };

            let affine = transform.affine();
            let vertex_data = vertex_data.chunks(2).map(|vs| {
                (
                    vsvg::Point::from(affine.transform_point3(Vec3::from(vs[0])).truncate()),
                    vsvg::Point::from(affine.transform_point3(Vec3::from(vs[1])).truncate()),
                )
            });

            doc.push_path(1, vsvg::Path::from_segments(vertex_data));
        }

        // convert to SVG coordinate system (y-axis down, origin top-left)
        doc.scale_non_uniform(1.0, -1.0);
        doc.translate(
            resolution.width as f64 / 2.0,
            resolution.height as f64 / 2.0,
        );
        doc.crop(0.0, 0.0, resolution.width as f64, resolution.height as f64);
        doc.push_path(
            2,
            kurbo::Rect::new(0.0, 0.0, resolution.width as f64, resolution.height as f64),
        );

        #[cfg(not(target_arch = "wasm32"))]
        {
            let file = std::io::BufWriter::new(std::fs::File::create("/tmp/output.svg").unwrap());
            doc.to_svg(file).unwrap();
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = doc
                .to_svg_string()
                .map(|svg| download_file("output.svg", &svg));
        }
    }
}

//https://stackoverflow.com/a/19328891/229511
#[cfg(target_arch = "wasm32")]
fn download_file(name: &str, content: &str) -> Option<()> {
    use wasm_bindgen::{JsCast, JsValue};
    use web_sys::{Blob, BlobPropertyBag, Url};

    let window = web_sys::window()?;
    let document = window.document()?;
    let body = document.body()?;

    let aa = document.create_element("a").ok()?;
    let a = aa.dyn_into::<web_sys::HtmlElement>().ok()?;
    a.style().set_property("display", "none").ok()?;
    body.append_child(&a).ok()?;

    let mut blob_options = BlobPropertyBag::new();
    blob_options.type_("image/svg+xml;charset=utf-8");

    let blob_sequence = js_sys::Array::new_with_length(1);
    blob_sequence.set(0, JsValue::from(content));

    let blob = Blob::new_with_blob_sequence_and_options(&blob_sequence, &blob_options).ok()?;

    let url = Url::create_object_url_with_blob(&blob).ok()?;

    a.set_attribute("href", &url).ok()?;
    a.set_attribute("download", name).ok()?;
    a.click();
    Url::revoke_object_url(&url).ok()?;
    a.remove();

    Some(())
}

impl Resolution {
    pub fn as_page_size(&self) -> vsvg::PageSize {
        vsvg::PageSize {
            w: self.width as f64,
            h: self.height as f64,
        }
    }
}
