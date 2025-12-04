use hdpictureconverter::Image;
use std::io::{Cursor, Write};
use wasm_bindgen::prelude::*;
use zip::ZipWriter;

#[wasm_bindgen]
extern "C" {
    pub type ProcessRequest;

    #[wasm_bindgen(method, getter)]
    pub fn image_name(this: &ProcessRequest) -> String;

    #[wasm_bindgen(method, getter)]
    pub fn image_data(this: &ProcessRequest) -> Box<[u8]>;

    #[wasm_bindgen(method, getter)]
    pub fn var_prefix(this: &ProcessRequest) -> String;
}

// This can't just be called 'Response': https://github.com/rustwasm/wasm-bindgen/issues/2470
#[wasm_bindgen]
pub struct ProcessResponse {
    #[wasm_bindgen(getter_with_clone)]
    pub zip_data: Box<[u8]>,
}

#[wasm_bindgen]
pub struct Converter;

#[wasm_bindgen]
impl Converter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Converter {
        Converter
    }

    pub fn process_message(
        &self,
        request: ProcessRequest,
        progress: &js_sys::Function,
    ) -> Result<ProcessResponse, String> {
        self.process_message_(request, progress)
            .map_err(|e| format!("{}", e))
    }

    fn process_message_(
        &self,
        request: ProcessRequest,
        progress: &js_sys::Function,
    ) -> Result<ProcessResponse, Box<dyn std::error::Error>> {
        let progress_stage = |stage: &str| {
            let _ = progress.call1(&JsValue::NULL, &stage.into());
        };
        let progress_count = |stage: &str, current: u32, total: u32| {
            let _ = progress.call3(
                &JsValue::NULL,
                &stage.into(),
                &current.into(),
                &total.into(),
            );
        };

        progress_stage("Decoding image");
        let im = Image::new(
            Cursor::new(request.image_data()),
            &request.image_name(),
            &request.var_prefix(),
        )?;

        progress_stage("Generating palette");
        let im = im.quantize();

        let now = js_sys::Date::new_0();
        let zip_options = zip::write::FileOptions::default().last_modified_time(
            zip::DateTime::from_date_and_time(
                now.get_full_year() as u16,
                (now.get_month() + 1) as u8,
                now.get_date() as u8,
                now.get_hours() as u8,
                now.get_minutes() as u8,
                now.get_seconds() as u8,
            )
            .map_err(|e| format!("Now ({now:?}) doesn't seem to be a valid time: {e:?}"))?,
        );

        // Dump tiles to appvars
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let n_tiles = im.width_tiles() * im.height_tiles();
        for (i, tile) in im.tiles().enumerate() {
            progress_count("Creating appvars", i as u32, n_tiles);

            // First write the appvar to a memory buffer, since we need to seek within it
            let var_data = tile.write_appvar(Cursor::new(Vec::new()))?.into_inner();

            // ..then write it into the zip
            zip.start_file(format!("{}.8xv", tile.appvar_name()), zip_options)?;
            zip.write_all(&var_data)?;
        }

        // Dump palette to appvar
        let palette_data = im
            .write_palette_appvar(Cursor::new(Vec::new()))?
            .into_inner();
        zip.start_file(format!("{}.8xv", im.palette_appvar_name()), zip_options)?;
        zip.write_all(&palette_data)?;

        let zip_bytes = zip.finish()?.into_inner();

        Ok(ProcessResponse {
            zip_data: zip_bytes.into_boxed_slice(),
        })
    }
}
