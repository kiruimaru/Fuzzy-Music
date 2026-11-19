# 0015: Custom player background

Date: 2026-11-19. Status: stacked on the album view branch; review pending.

The Settings view accepts a personal background image and an opacity slider
that applies while dragging. The chosen file is copied into the application
data directory under `background/` with a sanitised extension, and only that
copy is displayed; the original location is not referenced afterwards. Copying
keeps the background stable when the source file moves or is deleted, and it
lets the renderer stay inside one narrow asset scope instead of requesting
broad filesystem access. The dialog offers common image types and an all-files
filter, and the picker never transcodes: any image the WebView can decode
works, while unsupported formats simply produce no bitmap.

Settings schema version 4 adds `background_image` (empty means none) and
`background_opacity` (0–100, default 60). Saving validates that a non-empty
image path is absolute and exists, and that opacity stays in range; clearing
stores an empty string. The application enables Tauri's asset protocol with a
scope limited to the app data directory, and the CSP allows `asset:` and
`http://asset.localhost` in `img-src` alongside the existing SoundCloud hosts.

Rendering uses a fixed `body::before` layer driven by two CSS variables,
`--bg-image` and `--bg-opacity`, placed below the application root but above
the theme background colour, so panels keep their contrast. The slider updates
the variables on every input event for immediate feedback and persists the
value 300 ms after the drag settles; choosing or removing an image persists
immediately. Reopening the app restores both values from settings. A file that
disappears outside the app leaves the layer empty rather than breaking the
settings screen.

The feature adds no JavaScript dependencies. It enables the `protocol-asset`
Tauri feature, which brings the `http-range` crate, and loads the image from
local storage only.

While a background is set, the root element carries a `has-background` class and
the shell surfaces become frosted glass: the sidebar and the player bar render
translucent with a blurred, slightly saturated backdrop, so the artwork appears
continuous behind the navigation and transport instead of stopping at their
edges. Light theme uses its own translucent tones, and the active navigation
pill stays readable. Without a background the surfaces keep their original
opaque colours, so nothing changes for users who leave the feature alone.

GIF files are accepted like any other image: the picker copies the file without
transcoding, the extension is preserved as `background.gif`, and the asset
protocol serves it to the CSS layer, which keeps the animation running. The
settings text lists supported formats explicitly so animated backgrounds are
discoverable.

Animated backgrounds repaint every frame. While one is active the root carries
`has-animated-background`, the shell surfaces drop `backdrop-filter`, and their
tint becomes more opaque, so the animation cannot keep the GPU busy blurring
the whole window. Static images keep the frosted blur.
