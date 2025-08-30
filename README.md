<div align="center">
<img src="/crates/scrubkit-web/assets/logo.svg" alt="ScrubKit Logo" width="150" height="150">
<h1 style="font-family: 'Orbitron', sans-serif; font-size: 3rem;">ScrubKit</h1>
<p><strong>[ Anonymize Your Digital Footprint ]</strong></p>

[![Netlify Status](https://api.netlify.com/api/v1/badges/68c9ce0b-fc28-42b5-bfa7-d2c4f2e51680/deploy-status)](https://app.netlify.com/projects/scrubkit/deploys) [![Latest Release](https://img.shields.io/github/v/release/seahorse-byte/scrubkit)](https://github.com/seahorse-byte/scrubkit/releases/latest)

</div>

🛡️ Mission
ScrubKit is a modern, privacy-first tool designed to inspect and remove potentially sensitive metadata from your files. In a world where data privacy is paramount, ScrubKit provides a secure, transparent, and powerful way to ensure your files don't share more than you intend.

Built entirely in Rust and WebAssembly, it offers a blazing-fast experience for both terminal power-users and those who prefer a graphical interface, with the core promise that your files are never uploaded and never leave your machine.

✨ Features
Maximum Privacy: All processing happens locally on your machine, either in the terminal or directly in your browser via WebAssembly.

Current Support: Full view and scrub capabilities for JPEG and PNG files.

Future Support (View Only): Thanks to nom-exif, the core library can already parse and view metadata from a wider range of files, including HEIC, TIFF, MP4, and MOV. Full scrubbing support for these formats is planned for future releases.

Dual Interface:

💻 Powerful CLI: A robust command-line tool for scripting, automation, and quick actions.

🌐 Slick Web UI: An intuitive, dark-themed drag-and-drop interface for easy, visual use.

Built with Rust: High performance, memory safety, and reliability from the ground up.

Open Source: Trust through transparency. We invite the community to audit our code, contribute, and help us add support for even more file types.

🚀 Installation & Usage
You can use ScrubKit in two ways: through the command line or the web interface.

🌐 Web App
For a quick and visual experience, use the web application. It requires no installation.

➡️ Launch ScrubKit Web (Link will be live after deployment)

Open the link above.

Drag and drop a supported file (like a JPEG or PNG) into the dropzone.

View the discovered metadata.

Click "Scrub Metadata" and then "Download Anonymized File" to get a clean copy.

(Replace with a real screenshot of your app)

💻 Command-Line Interface (CLI)
For power users and automation, the CLI is the perfect tool.

Installation
Option 1: From Crates.io (Recommended)
If you have the Rust toolchain installed, you can install scrubkit directly from crates.io:

```rust
cargo install scrubkit
```

Option 2: From GitHub Releases
Alternatively, you can download a pre-compiled binary for your operating system from the Releases page.

Usage
The CLI is simple and intuitive.

View Metadata:

scrubkit view /path/to/your/photo.jpg

Output:

Metadata for /path/to/your/photo.jpg:
  - IFD0: Model = Test Model
  - IFD0: Make = Test Camera

Clean Metadata:
This creates a new file named photo.clean.jpg.

scrubkit clean /path/to/your/photo.jpg

Output:

Successfully removed 2 metadata entries.
Cleaned file saved to: /path/to/your/photo.clean.jpg

Clean Metadata In-Place:
To overwrite the original file (use with caution!):

scrubkit clean --in-place /path/to/your/document.png

🤝 Contributing
ScrubKit is an open-source project, and contributions are highly welcome! Whether it's adding support for a new file type, improving the UI, or fixing a bug, please feel free to open an issue or submit a pull request.

⚖️ License
This project is licensed under the MIT License. See the LICENSE file for details.
