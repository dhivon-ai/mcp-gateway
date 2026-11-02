# Dhivon shared brand assets

These production files are copied from Dhivon's approved Soft Gate brand kit. The
source selection preserves the original SVG paths, colors and aspect ratios. The
Manrope variable font is distributed with its SIL Open Font License in
`fonts/OFL.txt`. `USAGE.txt` retains the supplied usage instructions and provenance.

Import `theme.css` from the application stylesheet to bundle Manrope locally and
share the palette. Import the SVG or PNG used by a component from this directory
so the frontend build emits its own asset URL. Reference the supplied favicon
from the HTML entry. The public website and protected interfaces use this same
source set. Copying the whole source directory into a Rust runtime asset output
would also include source documentation, so let the frontend bundle select the
runtime files it references.

Use `dhivon.svg` for the outlined lowercase lockup at widths of at least 120 px.
Use `logo.svg` for smaller placements, with a minimum rendered mark of 16 px. The
`*-dark.svg` variants contain white lettering/enclosure with emerald for navy
surfaces. Preserve the two gaps, retain clear space of at least half the emerald
module width, and keep the logo free of shadows and gradients. `favicon.svg` and
`favicon.ico` use the kit's tighter padding for browser tabs. The touch icon has a
white background.

The navy action and white text pair has a 14.13:1 contrast ratio. Muted text on
white is 5.82:1, success text on white is 5.16:1 and border on white is 3.22:1.
Emerald against white is 2.90:1, so use the emerald square as a brand accent.
Use the darker success token when communicating a successful state, with an
explicit icon or label. Navy focus styling and control boundaries must remain
visible on the selected surface. Recheck any new foreground/background pair.

The shared file defines tokens only. Applications map these semantic values into
their existing component theme and apply the font to their own root element.
