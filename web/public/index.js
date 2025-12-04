let worker_ready = false;

const form = document.getElementById('inputsForm');
const imageInput = document.getElementById('imageInput');
const varPrefixInput = document.getElementById('varPrefixInput');
const submit = document.getElementById('submitButton');

const results = document.getElementById('results');
const downloadZipButton = document.getElementById('downloadZipButton');
const downloadZipLink = document.getElementById('downloadZipLink');

function showProgress(label, percent) {
    const text = document.getElementById('conversionText');
    const progress = document.getElementById('conversionProgress');

    if (percent !== undefined) {
        progress.value = percent;
    } else {
        // Make it indeterminate
        progress.removeAttribute('value');
    }
    text.textContent = label;
}

function updateReadiness() {
    submit.disabled = !worker_ready;
    submit.value = submit.disabled ? 'Not ready yet..' : 'Convert!';
}

/*
 * Handle the "Convert!" button by bundling up inputs and parameters
 * to the worker to actually process.
 *
 * We'll get a message back when it's done, handled in worker.onmessage
 */
form.onsubmit = async (e) => {
    // Browser validation ensures this is okay since we handle submit
    e.preventDefault();

    $('#statusModal').modal({keyboard: false});
    showProgress('Loading image');

    let file = imageInput.files[0];
    let image_data = await file.arrayBuffer();
    worker.postMessage({
        image_data: new Uint8Array(image_data),
        image_name: file.name,
        var_prefix: varPrefixInput.value,
    });
    console.log('Sent', image_data.byteLength, 'bytes to process');
};

/* Display the image selected in the form */
const preview = document.getElementById('preview');
preview.onload = () => {
    // Save memory by revoking the URL as soon as it's loaded
    URL.revokeObjectURL(preview.src);
};

function imageChanged() {
    updateReadiness();
    if (imageInput.files.length == 0) {
        preview.src = null;
    } else {
        preview.src = URL.createObjectURL(imageInput.files[0]);
    }
}

imageInput.onchange = imageChanged;

/* Handle drag+drop of image files */
function showDragActive(active) {
    let f;
    // Highlight the file input when drag is active
    if (active) {
        f = imageInput.classList.add("bg-info");
    } else {
        f = imageInput.classList.remove("bg-info");
    }
}

document.ondragenter = e => {
    if (e.dataTransfer.files) {
        e.dataTransfer.dropEffect = "copy";
        showDragActive(true);
    }
};
document.ondragover = e => {
    // Required to prevent default handling of drop as well
    e.preventDefault();
};
document.ondrop = e => {
    e.preventDefault();
    showDragActive(false);
    if (e.dataTransfer.files) {
        imageInput.files = e.dataTransfer.files;
        imageChanged();
    }
};
document.ondragend = document.ondragexit = e => {
    showDragActive(false);
};

/* Spawn the dedicated worker which actually runs the wasm app */
const worker = new Worker('worker.js');
worker.onmessage = (e) => {
    // Worker is only ready immediately after it sends a ready message;
    // anything else means it's busy.
    worker_ready = !!e.data.ready;
    updateReadiness();

    console.log('Received message from worker', e.data);
    if (e.data.error) {
        // Worker couldn't finish conversion for some reason
        showProgress('Failed: ' + e.data.error, 0);
    } else if (e.data.progress) {
        // Worker is reporting its ongoing progress
        const {kind, percent} = e.data.progress;
        showProgress(kind, percent);
    } else if (e.data.zip) {
        // Sent back results of conversion
        const {data, name} = e.data.zip;

        $(downloadZipButton).one('click', () => {
            downloadZipLink.href = URL.createObjectURL(new Blob([data]));
            downloadZipLink.download = name + '.zip'
            downloadZipLink.click();
            // click won't run synchronously, so let the event loop turn then
            // we can safely revoke the data URL to free the memory
            setTimeout(() => {
                downloadZipButton.disabled = true;
                URL.revokeObjectURL(downloadZipLink.href);
            }, 0);
        });
        downloadZipButton.disabled = false;
        showProgress('Conversion complete!', 1);
    }
};

/* Install a service worker if supported, so we're installable and work offline */
if ('serviceWorker' in navigator) {
    navigator.serviceWorker.register('serviceworker.js').then(reg => {
        // All this to handle prompting for updates..
        function listenForWaitingServiceWorker(reg, callback) {
            function awaitStateChange() {
                reg.installing.addEventListener('statechange', function () {
                    if (this.state === 'installed') callback(reg);
                });
            }

            if (!reg) return;
            if (reg.waiting) return callback(reg);
            if (reg.installing) awaitStateChange();
            reg.addEventListener('updatefound', awaitStateChange);
        }

        // reload once when the new Service Worker starts activating
        var refreshing;
        navigator.serviceWorker.addEventListener('controllerchange', function () {
                if (refreshing) {
                    return;
                }
                refreshing = true;
                window.location.reload();
            }
        );

        function promptUserToRefresh(reg) {
            document.getElementById('updateAppButton').addEventListener('click', () => {
                reg.waiting.postMessage('skipWaiting');
            });
            document.getElementById('hideToastButton').addEventListener('click', () => {
                $('#updateToast').toast('hide');
            });
            $('#updateToast').toast('show');
        }

        listenForWaitingServiceWorker(reg, promptUserToRefresh);
    })
}
