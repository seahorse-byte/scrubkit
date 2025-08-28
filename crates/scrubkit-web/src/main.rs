use dioxus::prelude::*;
use scrubkit_core::{MetadataEntry, Scrubber, jpeg::JpegScrubber};
use wasm_bindgen::JsCast;

// Define an enum for our application's state
#[derive(Clone, PartialEq)]
enum AppState {
    Idle,
    Loaded {
        file_name: String,
        metadata: Vec<MetadataEntry>,
    },
    Scrubbed {
        file_name: String,
        cleaned_bytes: Vec<u8>,
        metadata_removed: Vec<MetadataEntry>,
    },
    Error(String),
}

// Helper function to trigger a file download in the browser
fn download_bytes(file_name: &str, bytes: &[u8]) {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().unwrap();

    let blob = web_sys::Blob::new_with_u8_array_sequence(&js_sys::Array::of1(
        &js_sys::Uint8Array::from(bytes).into(),
    ))
    .unwrap();

    let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();

    let a = document
        .create_element("a")
        .unwrap()
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .unwrap();

    a.set_href(&url);
    a.set_download(file_name);
    body.append_child(&a).unwrap();
    a.click();
    body.remove_child(&a).unwrap();
    web_sys::Url::revoke_object_url(&url).unwrap();
}

fn app() -> Element {
    // Use signals for reactive state management
    let mut file_bytes = use_signal(|| None::<(String, Vec<u8>)>);
    let mut app_state = use_signal(|| AppState::Idle);

    // This effect runs whenever `file_bytes` changes
    use_effect(move || {
        if let Some((name, bytes)) = file_bytes() {
            log::info!("File loaded: {}, size: {}", name, bytes.len());
            match JpegScrubber::new(bytes) {
                Ok(scrubber) => match scrubber.view_metadata() {
                    Ok(metadata) => {
                        app_state.set(AppState::Loaded {
                            file_name: name,
                            metadata,
                        });
                    }
                    Err(e) => app_state.set(AppState::Error(e.to_string())),
                },
                Err(e) => app_state.set(AppState::Error(e.to_string())),
            }
        }
    });

    let handle_file_upload = move |evt: FormEvent| async move {
        if let Some(file_engine) = &evt.files() {
            let files = file_engine.files();
            if let Some(file_name) = files.first() {
                if let Some(file) = file_engine.read_file(file_name).await {
                    file_bytes.set(Some((file_name.clone(), file)));
                }
            }
        }
    };

    rsx! {
        div {
            class: "min-h-screen bg-gray-100 flex items-center justify-center p-4",
            div {
                class: "max-w-2xl w-full bg-white rounded-lg shadow-xl p-8 space-y-6",
                // Header
                div {
                    class: "text-center",
                    h1 { class: "text-4xl font-bold text-gray-800", "ScrubKit" }
                    p { class: "text-gray-500 mt-2", "View and remove metadata with privacy." }
                }

                // File Input
                div {
                    class: "flex flex-col items-center justify-center p-6 border-2 border-dashed border-gray-300 rounded-lg",
                    p { class: "text-gray-600 mb-4", "Select a JPEG file to get started" }
                    label {
                        class: "file-input-button",
                        "Select File"
                        input {
                            r#type: "file",
                            class: "hidden",
                            accept: ".jpg, .jpeg",
                            oninput: handle_file_upload,
                        }
                    }
                }

                // Main Content Area
                match app_state() {
                    AppState::Idle => {
                        rsx! { p { class: "text-center text-gray-500", "Your file's metadata will appear here." } }
                    },
                    AppState::Loaded { file_name, metadata } => {
                        rsx! {
                            div {
                                class: "space-y-4",
                                h3 { class: "text-xl font-semibold text-gray-700", "Metadata for ", span { class: "font-mono", "{file_name}" } }
                                if metadata.is_empty() {
                                    p { "No metadata found." }
                                } else {
                                    ul {
                                        class: "list-disc list-inside bg-gray-50 p-4 rounded-md max-h-60 overflow-y-auto",
                                        for entry in metadata {
                                            li { class: "font-mono text-sm",
                                                span { class: "font-semibold", "{entry.key}: " }
                                                "{entry.value}"
                                            }
                                        }
                                    }
                                }
                                button {
                                    class: "w-full bg-red-600 hover:bg-red-700 text-white font-bold py-2 px-4 rounded-md transition",
                                    onclick: move |_| {
                                        if let Some((name, bytes)) = file_bytes() {
                                            if let Ok(scrubber) = JpegScrubber::new(bytes) {
                                                if let Ok(result) = scrubber.scrub() {
                                                    app_state.set(AppState::Scrubbed {
                                                        file_name: name,
                                                        cleaned_bytes: result.cleaned_file_bytes,
                                                        metadata_removed: result.metadata_removed,
                                                    });
                                                }
                                            }
                                        }
                                    },
                                    "Scrub Metadata"
                                }
                            }
                        }
                    },
                    AppState::Scrubbed { file_name, cleaned_bytes, metadata_removed } => {
                        rsx! {
                            div {
                                class: "p-4 bg-green-100 border border-green-400 text-green-700 rounded-md text-center space-y-3",
                                h3 { class: "font-bold text-lg", "Scrubbing Successful!" }
                                p { "Removed {metadata_removed.len()} metadata entries from ", span { class: "font-mono", "{file_name}" } }
                                button {
                                    class: "w-full bg-green-600 hover:bg-green-700 text-white font-bold py-2 px-4 rounded-md transition",
                                    onclick: move |_| {
                                        let scrubbed_name = format!("{}.clean.jpg", file_name.strip_suffix(".jpg").unwrap_or(&file_name));
                                        download_bytes(&scrubbed_name, &cleaned_bytes);
                                    },
                                    "Download Scrubbed File"
                                }
                            }
                        }
                    },
                    AppState::Error(err) => {
                        rsx! { p { class: "text-red-500", "Error: {err}" } }
                    },
                }

                // Footer
                p {
                    class: "text-center text-xs text-gray-400 pt-4 border-t",
                    "All processing is done in your browser. Your files never leave your computer."
                }
            }
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    launch(app);
}
