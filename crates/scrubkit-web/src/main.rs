use dioxus::prelude::*;
use scrubkit_core::{MetadataEntry, scrubber_for_file};
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

#[component]
fn app() -> Element {
    // Use signals for reactive state management
    let mut file_bytes = use_signal(|| None::<(String, Vec<u8>)>);
    let mut app_state = use_signal(|| AppState::Idle);

    // This effect runs whenever `file_bytes` changes
    use_effect(move || {
        if let Some((name, bytes)) = file_bytes() {
            log::info!("File loaded: {}, size: {}", name, bytes.len());
            match scrubber_for_file(bytes) {
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
            class: "min-h-screen bg-gray-900 text-gray-300 flex items-center justify-center p-4 font-mono",
            div {
                class: "max-w-3xl w-full bg-gray-800/50 backdrop-blur-sm border border-green-500/20 rounded-lg shadow-2xl shadow-green-500/10 p-8 space-y-6",

                // Header with Logo
                div {
                    class: "flex flex-col items-center text-center",
                    // SVG Logo
                    svg {
                        class: "w-24 h-24 mb-4 text-green-400",
                        "viewBox": "0 0 24 24",
                        "fill": "none",
                        "stroke": "currentColor",
                        "stroke-width": "1.5",
                        "stroke-linecap": "round",
                        "stroke-linejoin": "round",
                        path { d: "M12 2L2 7V17L12 22L22 17V7L12 2Z" }
                        path { d: "M2 7L12 12L22 7" }
                        path { d: "M12 22V12" }
                        path { d: "M17 4.5L7 9.5", "stroke-width": "1", opacity: "0.5" }
                        path { d: "M20 7L10 12", "stroke-width": "1", opacity: "0.5" }
                        circle { cx: "12", cy: "12", r: "3", fill: "#f87171", stroke: "none", opacity: "0.8" }
                        path { d: "M10.5 13.5L13.5 10.5", stroke: "#111827", "stroke-width": "1.5" }
                    },
                    h1 { class: "text-5xl font-bold text-gray-100 font-orbitron", "ScrubKit" }
                    p { class: "text-green-400 mt-2", "[ View and Anonymize File Metadata ]" }
                }

                // File Input Dropzone
                label {
                    class: "flex flex-col items-center justify-center w-full p-6 border-2 border-dashed border-gray-600 hover:border-green-400 rounded-lg cursor-pointer transition-colors",
                    svg {
                        class: "w-10 h-10 text-gray-500 mb-3",
                        "viewBox": "0 0 24 24", "fill": "none", "stroke": "currentColor", "stroke-width": "2", "stroke-linecap": "round", "stroke-linejoin": "round",
                        path { d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" }
                        polyline { points: "17 8 12 3 7 8" }
                        line { x1: "12", y1: "3", x2: "12", y2: "15" }
                    }
                    p { class: "text-gray-400", "Drag & Drop or ", span { class: "font-semibold text-green-400", "Click to Select File" } }
                    input {
                        r#type: "file",
                        class: "hidden",
                        accept: ".jpg, .jpeg, .png",
                        oninput: handle_file_upload,
                    }
                }

                // Main Content Area
                match app_state() {
                    AppState::Idle => rsx! {
                        p { class: "text-center text-gray-500 animate-pulse", "Awaiting file..." }
                    },
                    AppState::Loaded { file_name, metadata } => rsx! {
                        div {
                            class: "space-y-4",
                            h3 { class: "text-xl font-semibold text-green-400", ":: Metadata for ", span { class: "font-orbitron", "{file_name}" } }
                            if metadata.is_empty() {
                                p { "No metadata found." }
                            } else {
                                div {
                                    class: "bg-gray-900/50 p-4 rounded-md max-h-60 overflow-y-auto border border-gray-700",
                                    for entry in metadata {
                                        p { class: "text-sm whitespace-pre-wrap",
                                            span { class: "text-green-400", "{entry.key}: " }
                                            span { class: "text-gray-300", "{entry.value}" }
                                        }
                                    }
                                }
                            }
                            button {
                                class: "w-full bg-red-600 hover:bg-red-700 text-white font-bold py-3 px-4 rounded-md transition-transform hover:scale-105",
                                onclick: move |_| {
                                    if let Some((name, bytes)) = file_bytes() {
                                        if let Ok(scrubber) = scrubber_for_file(bytes) {
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
                    },
                    AppState::Scrubbed { file_name, cleaned_bytes, metadata_removed } => rsx! {
                        div {
                            class: "p-4 bg-green-900/50 border border-green-500 text-green-300 rounded-md text-center space-y-3",
                            h3 { class: "font-bold text-lg font-orbitron", "Anonymization Complete" }
                            p { "Removed {metadata_removed.len()} metadata entries from ", span { class: "font-mono", "{file_name}" } }
                            button {
                                class: "w-full bg-green-600 hover:bg-green-700 text-white font-bold py-3 px-4 rounded-md transition-transform hover:scale-105",
                                onclick: move |_| {
                                    let scrubbed_name = format!("{}.clean.{}", file_name.strip_suffix(".jpg").unwrap_or(&file_name).strip_suffix(".jpeg").unwrap_or(&file_name).strip_suffix(".png").unwrap_or(&file_name), "jpg");
                                    download_bytes(&scrubbed_name, &cleaned_bytes);
                                },
                                "Download Anonymized File"
                            }
                        }
                    },
                    AppState::Error(err) => rsx! {
                        p { class: "text-red-400", "Error: {err}" }
                    },
                }

                // Footer
                p {
                    class: "text-center text-xs text-gray-500 pt-4 border-t border-gray-700",
                    "All processing is done client-side. Your files never leave your computer."
                }

            }
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    launch(app);
}
