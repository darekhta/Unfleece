# Third-Party Notices

Unfleece is licensed under AGPL-3.0-or-later. This file records bundled third-party
components whose licenses require prominent notice beyond package metadata.

## GhostPDL / Ghostscript WASM

- Package: `@okathira/ghostpdl-wasm`
- Version: 1.1.0
- License: AGPL-3.0-or-later
- Source: <https://github.com/okathira/ghostpdl-wasm>
- Upstream engine: GhostPDL / Ghostscript by Artifex Software
- Upstream source: <https://github.com/ArtifexSoftware/ghostpdl>

The package is used only in the browser-side Compress PDF tool. User files are written
to the Emscripten in-memory filesystem and are not uploaded by Unfleece.
