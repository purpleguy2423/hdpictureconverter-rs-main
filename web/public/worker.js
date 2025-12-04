importScripts('./pkg/hdpictureconverter_web.js');

const {Converter} = wasm_bindgen;

class ProcessRequest {
    get image_data() {
        return new Uint8Array([]);
    }

    get image_name() {
        return "test";
    }

    get var_prefix() {
        return "AA";
    }
}

async function init_wasm() {
    await wasm_bindgen('./pkg/hdpictureconverter_web_bg.wasm');

    var converter = new Converter();

    self.onmessage = e => {
        function progress(kind, complete, total) {
            let percent;
            if (complete !== undefined && total !== undefined) {
                percent = complete / total;
            } else {
                percent = undefined;
            }

            self.postMessage({ progress: {
                kind: kind,
                percent,
            }});
        }

        try {
            var response = converter.process_message(e.data, progress);
            self.postMessage({ zip: { name: e.data.image_name, data: response.zip_data }});
        } catch (e) {
            self.postMessage({ error: e });
        }
        self.postMessage({ ready: true });
    }

    self.postMessage({ ready: true });
}

init_wasm();
