# Android keyboard attribution

The top-right hint positioning in `DualKeyboardView.java` is adapted from
[HeliBoard v4.2 KeyboardView.java](https://github.com/HeliBorg/HeliBoard/blob/0c60dffb03a4e2e7a37cb82210f5299f93c03979/app/src/main/java/helium314/keyboard/keyboard/KeyboardView.java).
Upstream marks that file `Apache-2.0 AND GPL-3.0-only`:

Copyright (C) 2010 The Android Open Source Project; modified by HeliBoard contributors.
Tacit adapts the phone hint's measured glyph width and font-ascent positioning to
the framework KeyboardView and the existing Korean two-set legends.

License texts: [Apache License 2.0](licenses/Apache-2.0.txt),
[GNU GPL 3.0](licenses/GPL-3.0-only.txt).
Tacit's remaining code is covered by the repository's AGPL-3.0-only license.

HeliBoard's input engine, dictionaries, settings, and gesture libraries are not included.
The rounded key backgrounds and light/dark palettes are original Tacit resources.

## Lucide icons

The `app/res/drawable/ic_*.xml` vectors are adapted from
[Lucide](https://github.com/lucide-icons/lucide/tree/20aef31db93fe4d9f398dbb15b120d99c48eef95/icons)
at commit `20aef31db93fe4d9f398dbb15b120d99c48eef95`.
SVG geometry is preserved as Android vector paths with a 1.8-unit stroke.
Lucide uses ISC, with MIT attribution for icons derived from Feather.
The complete upstream [license](app/res/raw/lucide_license.txt) is bundled in the APK.
No Lucide runtime dependency is used. The toolbar logo is Tacit's existing `assets/icon.png`.
