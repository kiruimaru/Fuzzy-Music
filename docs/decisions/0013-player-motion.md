# 0013: Player motion

Date: 2026-11-19. Status: stacked on the artist pages branch; review pending.

The player gains a small CSS-only motion layer instead of a new animation
dependency. Shared tokens live in `:root`: `--motion-fast` 140 ms,
`--motion-base` 180 ms, `--motion-slow` 240 ms and `--ease-out`
`cubic-bezier(.2, .7, .3, 1)`, keeping every transition inside the requested
120–280 ms window without bounce.

Transport and row buttons transition background, border, colour and transform.
Hover lifts icon buttons by one pixel; pressing scales them down briefly, so
the press is visible without moving layout. The Play/Pause glyph swaps through
a keyed span that fades and scales in over 140 ms. Track-row hover and the
active-row highlight animate their background, so automatic advancement moves
the highlight smoothly instead of snapping.

The progress and volume sliders brighten their track on hover and while
dragging, and the thumb grows to 1.15 on hover and 1.3 with a soft accent ring
while dragged. Disabled sliders keep their reduced opacity and no pointer
feedback. The volume popover transitions opacity and transform in both
directions with a delayed visibility switch, so opening and closing are
animated without leaving an invisible layer over the player.

The queue panel enters with a 180 ms fade, rise and slight scale from its
bottom-right origin. It still unmounts immediately when closed: an exit
animation would keep a stale panel in the accessibility tree and delay rapid
toggle clicks, so only the opening is animated. The now-playing block is keyed
by the selected track URL; a track change fades and slides the artwork and
title in over 240 ms without touching playback state.

All effects use opacity, transform and colours only; none adds pointer-event
overlays, delays a state update or blocks clicks, so fast track switching and
panel toggling stay immediate. The `prefers-reduced-motion: reduce` media query
reduces animation and transition durations to 1 ms. Component structure and
existing styles stay in place apart from the keyed span and now-playing
wrapper.
