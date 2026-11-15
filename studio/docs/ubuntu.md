# Ubuntu desktop distribution

Target: Ubuntu Desktop 22.04 and 24.04, amd64. Install the release package with
`sudo apt install ./open-ontologies-studio_0.1.0_amd64.deb` (use the actual
artifact filename). APT resolves GTK, WebKitGTK and Python runtime libraries.
End users do not need Rust, Cargo, Node.js or npm: the engine and private Node
runtime are packaged with the app. A desktop session is required.

## Building (maintainers only)

Use Ubuntu 22.04 with the Rust toolchain pinned in CI and Node 24. Install the
Tauri build prerequisites listed in `.github/workflows/studio-ubuntu.yml`, then
run `bash studio/scripts/package-ubuntu.sh` from the repository root. Output is
under `studio/src-tauri/target/release/bundle/deb/`, including SHA256SUMS.
The workflow can also be dispatched from GitHub Actions.

## Rendering and data

2D and 3D both use Canvas 2D. 3D projection and layout are calculated on the CPU.
Drag to rotate in 3D, right-drag to pan, and scroll to zoom. The Tree view retains
the ontology hierarchy. Layout calculations run in a worker and stop after a
bounded number of iterations. The display currently limits input to the first
2,000 nodes and 8,000 candidate edges; neighborhood search is not implemented.

The Linux launcher disables accelerated WebKit compositing. No GPU or WebGL
context is required by the graph renderer. Verify this on real Ubuntu before
publishing a supported release.

Ingestion outputs are stored in the application's user data workspace; ontology
saves retain their existing `~/.open-ontologies` location. Ingestion uses Python
directly. Optional Presidio detection still requires its Python packages/models;
the existing detector warning describes reduced coverage when they are absent.
AI features require a configured provider. Graph viewing does not require one.

## Required release verification

Install the same artifact on clean 22.04 and 24.04 desktop VMs with GPU device
access/3D acceleration disabled. Test X11 and Wayland, launch from the desktop,
import/edit/save/reopen, both graph views, Tree, cached ingestion, port conflicts,
Unicode paths, shutdown and relaunch. Benchmark 2,000 nodes/8,000 edges on a
4-vCPU/8-GB VM (target 15 FPS while navigating). Confirm no developer tools or
system Node are installed. These VM and performance checks are not yet automated
by the package workflow; a package build alone does not certify compatibility.
