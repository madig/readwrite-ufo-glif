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

use norad::Glyph;
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

create_exception!(readwrite_ufo_glif, GlifReadError, PyException);

#[pyfunction]
fn read_glyph(
    glif_path: &str,
    glyph_object: Option<&PyAny>,
    point_pen: Option<&PyAny>,
) -> PyResult<()> {
    let glyph = Glyph::load(glif_path).map_err(|e| {
        GlifReadError::new_err(format!("Failed to read glif file at '{}': {}", glif_path, e))
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
    }

    dbg!(point_pen);

    Ok(())
}

#[pymodule]
fn readwrite_ufo_glif(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(read_glyph, m)?)?;

    m.add("GlifReadError", py.get_type::<GlifReadError>())?;

    Ok(())
}
