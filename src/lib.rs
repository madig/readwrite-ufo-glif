// Copyright 2021 Nikolaus Waxweiler
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashMap;

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::wrap_pyfunction;
use serde::Serialize;

create_exception!(readwrite_ufo_glif, GlifReadError, PyException);

#[pyfunction]
#[text_signature = "(layer_path, /)"]
fn read_layer(layer_path: &str) -> PyResult<Py<PyBytes>> {
    let mut layer = norad::Layer::load(&layer_path, "".into()).map_err(|e| {
        GlifReadError::new_err(format!("Failed to read layer at '{}': {}", layer_path, e))
    })?;

    let mut glyph_dicts: HashMap<String, GlyphDict> = HashMap::new();
    let glyph_names: Vec<String> = layer.iter().map(|g| String::from(&*g.name)).collect();
    for name in glyph_names.into_iter() {
        let glyph = std::sync::Arc::try_unwrap(layer.remove_glyph(&name).unwrap())
            .map_err(|_| GlifReadError::new_err(format!("Failed to extract glyph '{}'", &name)))?;
        let glyph_dict = convert_glyph(glyph).map_err(|e| {
            GlifReadError::new_err(format!("Failed to convert layer '{}': {}", layer_path, e))
        })?;
        glyph_dicts.insert(name, glyph_dict);
    }
    let serialized = serde_pickle::to_vec(&glyph_dicts, true).map_err(|e| {
        GlifReadError::new_err(format!("Failed to pickle layer '{}': {}", layer_path, e))
    })?;

    let gil = Python::acquire_gil();
    let py = gil.python();
    let bytes = unsafe { PyBytes::from_ptr(py, serialized.as_ptr(), serialized.len()) };
    let converted: Py<PyBytes> = Py::from(bytes);

    Ok(converted)
}

#[pyfunction]
#[text_signature = "(glif_path, /)"]
fn read_glyph(glif_path: &str) -> PyResult<Py<PyBytes>> {
    let glyph = norad::Glyph::load(&glif_path).map_err(|e| {
        GlifReadError::new_err(format!(
            "Failed to read glif file at '{}': {}",
            glif_path, e
        ))
    })?;

    let glyph_dict = convert_glyph(glyph)?;
    let serialized = serde_pickle::to_vec(&glyph_dict, true).map_err(|e| {
        GlifReadError::new_err(format!(
            "Failed to pickle glif file at '{}': {}",
            glif_path, e
        ))
    })?;

    let gil = Python::acquire_gil();
    let py = gil.python();
    let bytes = unsafe { PyBytes::from_ptr(py, serialized.as_ptr(), serialized.len()) };
    let converted: Py<PyBytes> = Py::from(bytes);

    Ok(converted)
}

#[pymodule]
fn readwrite_ufo_glif(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(read_layer, m)?)?;
    m.add_function(wrap_pyfunction!(read_glyph, m)?)?;

    m.add("GlifReadError", py.get_type::<GlifReadError>())?;

    Ok(())
}

#[derive(Serialize)]
struct GlyphDict {
    #[serde(skip_serializing_if = "Option::is_none")]
    unicodes: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "f32_is_zero")]
    height: f32,
    #[serde(skip_serializing_if = "f32_is_zero")]
    width: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<ImageDict>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    anchors: Vec<AnchorDict>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    guidelines: Vec<GuidelineDict>,
    #[serde(skip_serializing_if = "dict_is_empty")]
    lib: serde_json::Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    contours: Vec<ContourDict>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    components: Vec<ComponentDict>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Serialize)]
struct ImageDict {
    #[serde(rename = "fileName")]
    file_name: String,
    #[serde(skip_serializing_if = "transform_is_identity")]
    transformation: (f32, f32, f32, f32, f32, f32),
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
}

#[derive(Serialize)]
struct AnchorDict {
    name: Option<String>,
    x: f32,
    y: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    identifier: Option<String>,
}

#[derive(Serialize)]
struct GuidelineDict {
    #[serde(skip_serializing_if = "Option::is_none")]
    x: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    y: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    angle: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    identifier: Option<String>,
}

#[derive(Serialize)]
struct ContourDict {
    points: Vec<PointDict>,
    #[serde(skip_serializing_if = "Option::is_none")]
    identifier: Option<String>,
}

#[derive(Serialize)]
struct PointDict {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    x: f32,
    y: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    smooth: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    identifier: Option<String>,
}

#[derive(Serialize)]
struct ComponentDict {
    #[serde(rename = "baseGlyph")]
    base_glyph: String,
    #[serde(skip_serializing_if = "transform_is_identity")]
    transformation: (f32, f32, f32, f32, f32, f32),
    #[serde(skip_serializing_if = "Option::is_none")]
    identifier: Option<String>,
}

fn dict_is_empty(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Object(d) => d.is_empty(),
        _ => unreachable!(),
    }
}

fn is_false(b: &bool) -> bool {
    b == &false
}

fn f32_is_zero(v: &f32) -> bool {
    v == &0.0
}

fn transform_is_identity(transformation: &(f32, f32, f32, f32, f32, f32)) -> bool {
    transformation == &(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
}

fn convert_glyph(glyph: norad::Glyph) -> PyResult<GlyphDict> {
    let unicodes: Option<Vec<u32>> = if !glyph.codepoints.is_empty() {
        Some(glyph.codepoints.iter().map(|c| *c as u32).collect())
    } else {
        None
    };
    let image: Option<ImageDict> = glyph.image.as_ref().map(|i| ImageDict {
        file_name: i.file_name.to_string_lossy().into(),
        transformation: (
            i.transform.x_scale,
            i.transform.xy_scale,
            i.transform.yx_scale,
            i.transform.y_scale,
            i.transform.x_offset,
            i.transform.y_offset,
        ),
        color: i.color.as_ref().map(|c| c.to_rgba_string()),
    });
    let anchors: Vec<AnchorDict> = glyph
        .anchors
        .iter()
        .map(|a| AnchorDict {
            name: a.name.clone(),
            x: a.x,
            y: a.y,
            color: a.color.as_ref().map(|c| c.to_rgba_string()),
            identifier: a.identifier().as_ref().map(|c| String::from(c.as_str())),
        })
        .collect();
    let guidelines: Vec<GuidelineDict> = glyph
        .guidelines
        .iter()
        .map(|g| match g.line {
            norad::Line::Vertical(x) => GuidelineDict {
                x: Some(x),
                y: None,
                angle: None,
                name: g.name.clone(),
                color: g.color.as_ref().map(|c| c.to_rgba_string()),
                identifier: g.identifier().as_ref().map(|c| String::from(c.as_str())),
            },
            norad::Line::Horizontal(y) => GuidelineDict {
                x: None,
                y: Some(y),
                angle: None,
                name: g.name.clone(),
                color: g.color.as_ref().map(|c| c.to_rgba_string()),
                identifier: g.identifier().as_ref().map(|c| String::from(c.as_str())),
            },
            norad::Line::Angle { x, y, degrees } => GuidelineDict {
                x: Some(x),
                y: Some(y),
                angle: Some(degrees),
                name: g.name.clone(),
                color: g.color.as_ref().map(|c| c.to_rgba_string()),
                identifier: g.identifier().as_ref().map(|c| String::from(c.as_str())),
            },
        })
        .collect();

    let mut contours: Vec<ContourDict> = Vec::new();
    for contour in &glyph.contours {
        let points: Vec<PointDict> = contour
            .points
            .iter()
            .map(|point| PointDict {
                name: point.name.clone(),
                x: point.x,
                y: point.y,
                r#type: match point.typ {
                    norad::PointType::Move => Some("move".into()),
                    norad::PointType::Line => Some("line".into()),
                    norad::PointType::OffCurve => None,
                    norad::PointType::Curve => Some("curve".into()),
                    norad::PointType::QCurve => Some("qcurve".into()),
                },
                smooth: point.smooth,
                identifier: point
                    .identifier()
                    .as_ref()
                    .map(|c| String::from(c.as_str())),
            })
            .collect();
        contours.push(ContourDict {
            points: points,
            identifier: contour
                .identifier()
                .as_ref()
                .map(|c| String::from(c.as_str())),
        });
    }
    let components: Vec<ComponentDict> = glyph
        .components
        .iter()
        .map(|c| ComponentDict {
            base_glyph: c.base.to_string(),
            transformation: (
                c.transform.x_scale,
                c.transform.xy_scale,
                c.transform.yx_scale,
                c.transform.y_scale,
                c.transform.x_offset,
                c.transform.y_offset,
            ),
            identifier: c.identifier().as_ref().map(|c| String::from(c.as_str())),
        })
        .collect();

    let object_libs = dump_object_libs(&glyph);
    let mut lib = glyph.lib;
    lib.insert(
        "public.objectLibs".into(),
        plist::Value::Dictionary(object_libs),
    );

    let dict = plist::Value::Dictionary(lib.clone());
    let mut cursor = std::io::Cursor::new(Vec::new());
    dict.to_writer_binary(&mut cursor).unwrap();
    cursor.set_position(0);
    let data = plist::from_reader::<&mut std::io::Cursor<Vec<u8>>, serde_json::Value>(&mut cursor)
        .map_err(|e| GlifReadError::new_err(format!("Failed to convert lib: {}", e)))?;

    Ok(GlyphDict {
        unicodes,
        height: glyph.height,
        width: glyph.width,
        image,
        anchors,
        guidelines,
        lib: data,
        contours,
        components,
        note: glyph.note,
    })
}

fn dump_object_libs(glyph: &norad::Glyph) -> norad::Plist {
    let mut object_libs = norad::Plist::default();

    let mut dump_lib = |id: Option<&norad::Identifier>, lib: &norad::Plist| {
        let id = id.map(|id| id.as_str().to_string());
        object_libs.insert(id.unwrap(), plist::Value::Dictionary(lib.clone()));
    };

    for anchor in &glyph.anchors {
        if let Some(lib) = anchor.lib() {
            dump_lib(anchor.identifier(), lib);
        }
    }

    for guideline in &glyph.guidelines {
        if let Some(lib) = guideline.lib() {
            dump_lib(guideline.identifier(), lib);
        }
    }

    for contour in &glyph.contours {
        if let Some(lib) = contour.lib() {
            dump_lib(contour.identifier(), lib);
        }
        for point in &contour.points {
            if let Some(lib) = point.lib() {
                dump_lib(point.identifier(), lib);
            }
        }
    }
    for component in &glyph.components {
        if let Some(lib) = component.lib() {
            dump_lib(component.identifier(), lib);
        }
    }

    object_libs
}
