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

use norad::Glyph;
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::wrap_pyfunction;

create_exception!(readwrite_ufo_glif, GlifReadError, PyException);

#[pyfunction]
fn read_glyph(
    glif_path: &str,
    glyph_object: Option<&PyAny>,
    point_pen: Option<&PyAny>,
) -> PyResult<()> {
    let glyph = Glyph::load(glif_path).map_err(|e| {
        GlifReadError::new_err(format!(
            "Failed to read glif file at '{}': {}",
            glif_path, e
        ))
    })?;

    let gil = Python::acquire_gil();
    let py = gil.python();

    if let Some(glyph_object) = glyph_object {
        if !glyph.codepoints.is_empty() {
            let codepoints: Vec<u32> = glyph.codepoints.iter().map(|c| *c as u32).collect();
            glyph_object.setattr("unicodes", codepoints.to_object(py))?;
        }

        if glyph.height != 0.0 {
            glyph_object.setattr("height", glyph.height.to_object(py))?;
        }
        if glyph.width != 0.0 {
            glyph_object.setattr("width", glyph.width.to_object(py))?;
        }

        if let Some(image) = &glyph.image {
            let kwargs = [
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
            .into_py_dict(py);
            glyph_object.setattr("image", kwargs)?;
        }

        if !glyph.anchors.is_empty() {
            let args: Vec<_> = glyph
                .anchors
                .iter()
                .map(|a| {
                    [
                        ("name", a.name.to_object(py)),
                        ("x", a.x.to_object(py)),
                        ("y", a.y.to_object(py)),
                        (
                            "color",
                            a.color.as_ref().map(|c| c.to_rgba_string()).to_object(py),
                        ),
                        (
                            "identifier",
                            a.identifier().as_ref().map(|c| c.as_str()).to_object(py),
                        ),
                    ]
                    .into_py_dict(py)
                })
                .collect();
            glyph_object.setattr("anchors", args.to_object(py))?;
        }

        if !glyph.guidelines.is_empty() {
            let args: Vec<_> = glyph
                .guidelines
                .iter()
                .map(|g| {
                    let (x, y, angle) = match g.line {
                        norad::Line::Vertical(x) => (Some(x), None, None),
                        norad::Line::Horizontal(y) => (None, Some(y), None),
                        norad::Line::Angle { x, y, degrees } => (Some(x), Some(y), Some(degrees)),
                    };
                    [
                        ("name", g.name.to_object(py)),
                        ("x", x.to_object(py)),
                        ("y", y.to_object(py)),
                        ("angle", angle.to_object(py)),
                        (
                            "color",
                            g.color.as_ref().map(|c| c.to_rgba_string()).to_object(py),
                        ),
                        (
                            "identifier",
                            g.identifier().as_ref().map(|c| c.as_str()).to_object(py),
                        ),
                    ]
                    .into_py_dict(py)
                })
                .collect();
            glyph_object.setattr("guidelines", args.to_object(py))?;
        }

        // Convert the glyph lib.
        let mut glyph_lib = HashMap::<&str, PyObject>::new();
        for (key, value) in glyph.lib.iter() {
            let py_value = convert_lib_key_value(key, value, py).map_err(|e| {
                GlifReadError::new_err(format!(
                    "Failed to read glif file at '{}' due to glyph lib data: {}",
                    glif_path, e
                ))
            })?;
            glyph_lib.insert(key, py_value);
        }

        // Look for object libs to fill in.
        let mut object_libs = HashMap::<&str, PyObject>::new();
        for anchor in &glyph.anchors {
            if let Some(olib) = anchor.lib() {
                let mut object_lib = HashMap::<&str, PyObject>::new();
                for (key, value) in olib.iter() {
                    let py_value = convert_lib_key_value(key, value, py).map_err(|e| {
                        GlifReadError::new_err(format!(
                            "Failed to read glif file at '{}' due to anchor lib data: {}",
                            glif_path, e
                        ))
                    })?;
                    object_lib.insert(key, py_value);
                }
                object_libs.insert(
                    anchor.identifier().unwrap().as_str(),
                    object_lib.to_object(py),
                );
            }
        }
        for guideline in &glyph.guidelines {
            if let Some(olib) = guideline.lib() {
                let mut object_lib = HashMap::<&str, PyObject>::new();
                for (key, value) in olib.iter() {
                    let py_value = convert_lib_key_value(key, value, py).map_err(|e| {
                        GlifReadError::new_err(format!(
                            "Failed to read glif file at '{}' due to guideline lib data: {}",
                            glif_path, e
                        ))
                    })?;
                    object_lib.insert(key, py_value);
                }
                object_libs.insert(
                    guideline.identifier().unwrap().as_str(),
                    object_lib.to_object(py),
                );
            }
        }
        for contour in &glyph.contours {
            if let Some(olib) = contour.lib() {
                let mut object_lib = HashMap::<&str, PyObject>::new();
                for (key, value) in olib.iter() {
                    let py_value = convert_lib_key_value(key, value, py).map_err(|e| {
                        GlifReadError::new_err(format!(
                            "Failed to read glif file at '{}' due to contour lib data: {}",
                            glif_path, e
                        ))
                    })?;
                    object_lib.insert(key, py_value);
                }
                object_libs.insert(
                    contour.identifier().unwrap().as_str(),
                    object_lib.to_object(py),
                );
            }
            for point in &contour.points {
                if let Some(olib) = point.lib() {
                    let mut object_lib = HashMap::<&str, PyObject>::new();
                    for (key, value) in olib.iter() {
                        let py_value = convert_lib_key_value(key, value, py).map_err(|e| {
                            GlifReadError::new_err(format!(
                                "Failed to read glif file at '{}' due to point lib data: {}",
                                glif_path, e
                            ))
                        })?;
                        object_lib.insert(key, py_value);
                    }
                    object_libs.insert(
                        point.identifier().unwrap().as_str(),
                        object_lib.to_object(py),
                    );
                }
            }
        }
        for component in &glyph.components {
            if let Some(olib) = component.lib() {
                let mut object_lib = HashMap::<&str, PyObject>::new();
                for (key, value) in olib.iter() {
                    let py_value = convert_lib_key_value(key, value, py).map_err(|e| {
                        GlifReadError::new_err(format!(
                            "Failed to read glif file at '{}' due to component lib data: {}",
                            glif_path, e
                        ))
                    })?;
                    object_lib.insert(key, py_value);
                }
                object_libs.insert(
                    component.identifier().unwrap().as_str(),
                    object_lib.to_object(py),
                );
            }
        }

        if !object_libs.is_empty() {
            glyph_lib.insert("public.objectLibs", object_libs.to_object(py));
        }
        if !glyph_lib.is_empty() {
            glyph_object.setattr("lib", glyph_lib.into_py_dict(py))?;
        }

        if let Some(note) = &glyph.note {
            glyph_object.setattr("note", note.to_object(py))?;
        }
    }

    if let Some(point_pen) = point_pen {
        for contour in &glyph.contours {
            let kwargs = [(
                "identifier",
                contour
                    .identifier()
                    .as_ref()
                    .map(|c| c.as_str())
                    .to_object(py),
            )]
            .into_py_dict(py);
            point_pen.call_method("beginPath", (), Some(kwargs))?;

            for point in &contour.points {
                let kwargs = [
                    ("name", point.name.to_object(py)),
                    ("pt", (point.x, point.y).to_object(py)),
                    // ("x", point.x.to_object(py)),
                    // ("y", point.y.to_object(py)),
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
                .into_py_dict(py);
                point_pen.call_method("addPoint", (), Some(kwargs))?;
            }

            point_pen.call_method("endPath", (), None)?;
        }

        for component in &glyph.components {
            let kwargs = [
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
            .into_py_dict(py);
            point_pen.call_method("addComponent", (), Some(kwargs))?;
        }
    }

    Ok(())
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

#[pymodule]
fn readwrite_ufo_glif(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(read_glyph, m)?)?;

    m.add("GlifReadError", py.get_type::<GlifReadError>())?;

    Ok(())
}
