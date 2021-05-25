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
use pyo3::types::IntoPyDict;
use pyo3::wrap_pyfunction;

create_exception!(readwrite_ufo_glif, GlifReadError, PyException);

#[pyfunction]
fn read_layer(layer_path: &str) -> PyResult<HashMap<String, PyObject>> {
    let layer = norad::Layer::load(&layer_path, "".into()).map_err(|e| {
        GlifReadError::new_err(format!("Failed to read layer at '{}': {}", layer_path, e))
    })?;

    let mut dicts: HashMap<String, PyObject> = HashMap::new();
    let gil = Python::acquire_gil();
    let py = gil.python();
    for glyph in layer.iter().map(|g| g.as_ref()) {
        let glyph_dict = convert_glyph(glyph, py)?;
        dicts.insert(glyph.name.to_string(), glyph_dict);
    }

    Ok(dicts)
}

#[pyfunction]
fn read_glyph(glif_path: &str) -> PyResult<PyObject> {
    let glyph = norad::Glyph::load(&glif_path).map_err(|e| {
        GlifReadError::new_err(format!("Failed to read glif file at '{}': {}", glif_path, e))
    })?;

    let gil = Python::acquire_gil();
    let py = gil.python();
    let glyph_dict = convert_glyph(&glyph, py)?;

    Ok(glyph_dict)
}

#[pymodule]
fn readwrite_ufo_glif(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(read_layer, m)?)?;
    m.add_function(wrap_pyfunction!(read_glyph, m)?)?;

    m.add("GlifReadError", py.get_type::<GlifReadError>())?;

    Ok(())
}

fn convert_glyph(glyph: &norad::Glyph, py: Python) -> PyResult<PyObject> {
    let mut glyph_dict: HashMap<&str, PyObject> = HashMap::new();
    if !glyph.codepoints.is_empty() {
        let codepoints: Vec<u32> = glyph.codepoints.iter().map(|c| *c as u32).collect();
        glyph_dict.insert("unicodes", codepoints.to_object(py));
    }
    if glyph.height != 0.0 {
        glyph_dict.insert("height", glyph.height.to_object(py));
    }
    if glyph.width != 0.0 {
        glyph_dict.insert("width", glyph.width.to_object(py));
    }
    if let Some(image) = &glyph.image {
        let kwargs = convert_image(image, py);
        glyph_dict.insert("image", kwargs.to_object(py));
    }
    if !glyph.anchors.is_empty() {
        let args: Vec<_> = glyph
            .anchors
            .iter()
            .map(|a| convert_anchor(a, py))
            .collect();
        glyph_dict.insert("anchors", args.to_object(py));
    }
    if !glyph.guidelines.is_empty() {
        let args: Vec<_> = glyph
            .guidelines
            .iter()
            .map(|g| convert_guideline(g, py))
            .collect();
        glyph_dict.insert("guidelines", args.to_object(py));
    }
    let mut glyph_lib = HashMap::<&str, PyObject>::new();
    for (key, value) in glyph.lib.iter() {
        let py_value = convert_lib_key_value(key, value, py).map_err(|e| {
            GlifReadError::new_err(format!(
                "Failed to convert lib data for glyph '{}': {}",
                &glyph.name, e
            ))
        })?;
        glyph_lib.insert(key, py_value);
    }
    let mut object_libs = HashMap::<&str, PyObject>::new();
    for anchor in &glyph.anchors {
        if let Some(olib) = anchor.lib() {
            let object_lib = convert_object_lib(olib, py).map_err(|e| {
                GlifReadError::new_err(format!(
                    "Failed to convert lib data for glyph '{}': {}",
                    &glyph.name, e
                ))
            })?;
            object_libs.insert(anchor.identifier().unwrap().as_str(), object_lib);
        }
    }
    for guideline in &glyph.guidelines {
        if let Some(olib) = guideline.lib() {
            let object_lib = convert_object_lib(olib, py).map_err(|e| {
                GlifReadError::new_err(format!(
                    "Failed to convert lib data for glyph '{}': {}",
                    &glyph.name, e
                ))
            })?;
            object_libs.insert(guideline.identifier().unwrap().as_str(), object_lib);
        }
    }
    for contour in &glyph.contours {
        if let Some(olib) = contour.lib() {
            let object_lib = convert_object_lib(olib, py).map_err(|e| {
                GlifReadError::new_err(format!(
                    "Failed to convert lib data for glyph '{}': {}",
                    &glyph.name, e
                ))
            })?;
            object_libs.insert(contour.identifier().unwrap().as_str(), object_lib);
        }
        for point in &contour.points {
            if let Some(olib) = point.lib() {
                let object_lib = convert_object_lib(olib, py).map_err(|e| {
                    GlifReadError::new_err(format!(
                        "Failed to convert lib data for glyph '{}': {}",
                        &glyph.name, e
                    ))
                })?;
                object_libs.insert(point.identifier().unwrap().as_str(), object_lib);
            }
        }
    }
    for component in &glyph.components {
        if let Some(olib) = component.lib() {
            let object_lib = convert_object_lib(olib, py).map_err(|e| {
                GlifReadError::new_err(format!(
                    "Failed to convert lib data for glyph '{}': {}",
                    &glyph.name, e
                ))
            })?;
            object_libs.insert(component.identifier().unwrap().as_str(), object_lib);
        }
    }
    if !object_libs.is_empty() {
        glyph_lib.insert("public.objectLibs", object_libs.to_object(py));
    }
    if !glyph_lib.is_empty() {
        glyph_dict.insert("lib", glyph_lib.into_py_dict(py).to_object(py));
    }
    if let Some(note) = &glyph.note {
        glyph_dict.insert("note", note.to_object(py));
    }
    let mut contours: Vec<PyObject> = Vec::new();
    for contour in &glyph.contours {
        let points: Vec<PyObject> = contour
            .points
            .iter()
            .map(|point| convert_point(point, py))
            .collect();
        let contour = convert_contour(contour, points, py);
        contours.push(contour);
    }
    glyph_dict.insert("contours", contours.to_object(py));
    let components: Vec<PyObject> = glyph
        .components
        .iter()
        .map(|c| convert_component(c, py))
        .collect();
    glyph_dict.insert("components", components.to_object(py));

    Ok(glyph_dict.to_object(py))
}

fn convert_anchor(anchor: &norad::Anchor, py: Python) -> PyObject {
    [
        ("name", anchor.name.to_object(py)),
        ("x", anchor.x.to_object(py)),
        ("y", anchor.y.to_object(py)),
        (
            "color",
            anchor
                .color
                .as_ref()
                .map(|c| c.to_rgba_string())
                .to_object(py),
        ),
        (
            "identifier",
            anchor
                .identifier()
                .as_ref()
                .map(|c| c.as_str())
                .to_object(py),
        ),
    ]
    .into_py_dict(py)
    .to_object(py)
}

fn convert_guideline(guideline: &norad::Guideline, py: Python) -> PyObject {
    let (x, y, angle) = match guideline.line {
        norad::Line::Vertical(x) => (Some(x), None, None),
        norad::Line::Horizontal(y) => (None, Some(y), None),
        norad::Line::Angle { x, y, degrees } => (Some(x), Some(y), Some(degrees)),
    };
    [
        ("name", guideline.name.to_object(py)),
        ("x", x.to_object(py)),
        ("y", y.to_object(py)),
        ("angle", angle.to_object(py)),
        (
            "color",
            guideline
                .color
                .as_ref()
                .map(|c| c.to_rgba_string())
                .to_object(py),
        ),
        (
            "identifier",
            guideline
                .identifier()
                .as_ref()
                .map(|c| c.as_str())
                .to_object(py),
        ),
    ]
    .into_py_dict(py)
    .to_object(py)
}

fn convert_image(image: &norad::Image, py: Python) -> PyObject {
    [
        ("fileName", image.file_name.to_string_lossy().to_object(py)),
        ("xScale", image.transform.x_scale.to_object(py)),
        ("xyScale", image.transform.xy_scale.to_object(py)),
        ("yxScale", image.transform.yx_scale.to_object(py)),
        ("yScale", image.transform.y_scale.to_object(py)),
        ("xOffset", image.transform.x_offset.to_object(py)),
        ("yOffset", image.transform.y_offset.to_object(py)),
        (
            "color",
            image
                .color
                .as_ref()
                .map(|c| c.to_rgba_string())
                .to_object(py),
        ),
    ]
    .into_py_dict(py)
    .to_object(py)
}

fn convert_contour(contour: &norad::Contour, points: Vec<PyObject>, py: Python) -> PyObject {
    [
        ("points", points.to_object(py)),
        (
            "identifier",
            contour
                .identifier()
                .as_ref()
                .map(|c| c.as_str())
                .to_object(py),
        ),
    ]
    .into_py_dict(py)
    .to_object(py)
}

fn convert_point(point: &norad::ContourPoint, py: Python) -> PyObject {
    [
        ("name", point.name.to_object(py)),
        ("x", point.x.to_object(py)),
        ("y", point.y.to_object(py)),
        (
            "type",
            match point.typ {
                norad::PointType::Move => Some("move"),
                norad::PointType::Line => Some("line"),
                norad::PointType::OffCurve => None,
                norad::PointType::Curve => Some("curve"),
                norad::PointType::QCurve => Some("qcurve"),
            }
            .to_object(py),
        ),
        ("smooth", point.smooth.to_object(py)),
        (
            "identifier",
            point
                .identifier()
                .as_ref()
                .map(|c| c.as_str())
                .to_object(py),
        ),
    ]
    .into_py_dict(py)
    .to_object(py)
}

fn convert_component(component: &norad::Component, py: Python) -> PyObject {
    [
        ("baseGlyph", component.base.to_object(py)),
        (
            "transformation",
            (
                component.transform.x_scale,
                component.transform.xy_scale,
                component.transform.yx_scale,
                component.transform.y_scale,
                component.transform.x_offset,
                component.transform.y_offset,
            )
                .to_object(py),
        ),
        (
            "identifier",
            component
                .identifier()
                .as_ref()
                .map(|c| c.as_str())
                .to_object(py),
        ),
    ]
    .into_py_dict(py)
    .to_object(py)
}

fn convert_object_lib(olib: &plist::Dictionary, py: Python) -> PyResult<PyObject> {
    let mut object_lib = HashMap::<&str, PyObject>::new();
    for (key, value) in olib.iter() {
        let py_value = convert_lib_key_value(key, value, py)?;
        object_lib.insert(key, py_value);
    }
    Ok(object_lib.into_py_dict(py).to_object(py))
}

fn convert_lib_key_value(key: &str, value: &plist::Value, py: Python) -> PyResult<PyObject> {
    Ok(match value {
        plist::Value::String(s) => s.to_object(py),
        plist::Value::Array(a) => {
            let mut py_a: Vec<PyObject> = Vec::new();
            for v in a.iter() {
                py_a.push(convert_lib_key_value(key, v, py)?)
            }
            py_a.to_object(py)
        }
        plist::Value::Dictionary(d) => {
            let mut py_d = HashMap::<&str, PyObject>::new();
            for (k, v) in d.iter() {
                py_d.insert(k, convert_lib_key_value(key, v, py)?);
            }
            py_d.to_object(py)
        }
        plist::Value::Boolean(b) => b.to_object(py),
        plist::Value::Data(d) => d.to_object(py),
        // plist::Value::Date(d) => {
        //     let date: std::time::SystemTime = d.into();
        // }
        plist::Value::Real(r) => r.to_object(py),
        plist::Value::Integer(i) => {
            if let Some(i) = i.as_signed() {
                i.to_object(py)
            } else if let Some(i) = i.as_unsigned() {
                i.to_object(py)
            } else {
                return Err(PyException::new_err(format!(
                    "lib element contains unconvertible integer for key '{}'",
                    key
                )));
            }
        }
        _ => {
            return Err(PyException::new_err(format!(
                "lib element contains unhandled data format for key '{}'",
                key
            )))
        }
    })
}
