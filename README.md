# Font Manager

A cross-platform font manager: browse every font you have, preview any face without
installing it, and switch individual styles on and off. Rust core, Tauri 2 shell,
Svelte 5 frontend.

## Running it

```sh
npm install
npm run tauri dev        # development, hot reload
npm run tauri build      # release bundle for the current OS
```

Linux needs WebKitGTK first: `apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev`.
macOS and Windows need nothing beyond Rust and Node.

On first launch the app indexes the system font folders so there is something to
browse. Those fonts are marked **installed**: the OS owns them and nothing can
deactivate them. Use **Add folder…** to add your own library — fonts there are the
ones you switch on and off.

### Google Fonts

Google Fonts appears in the sidebar. It needs a free API key from Google;
**Settings** explains how to get one, with links to the right pages in Google
Cloud. Each family previews in its own typeface, and **Download** saves it into a
managed library folder, after which it can be activated like any other font.
Downloaded families show **Update** when Google publishes a new version.

It has the library's view options: the same sample text, preview size, and
list or grid. With no sample typed, each family shows its name from a tiny
name-only file; once you type, rows load the family's regular style in full,
fetched and cached by the Rust side as the row comes into view. The arrow
beside a family opens a closer look: a waterfall, the alphabet and figures,
and its category, styles, axes, scripts, tags and version.

**Activate** downloads a family and switches it on in one step. Once a family
is in your library, its row shows the same on/off circle as in My library, so
it can be switched without leaving Google Fonts.

How the integration behaves:

- **The key** is checked with Google before it's saved. A key Google rejects is
  not saved, with a message saying why (wrong key, or the API isn't enabled for
  its project). If Google can't be reached, the key is saved with a warning.
- **It's stored in the OS keychain** (Keychain, Credential Manager, Secret
  Service). With no keychain available it falls back to a file readable only by
  your user, and Settings says which one was used. The UI only ever sees the
  key's last four characters, and errors are scrubbed of it.
- **The webview never contacts Google.** The catalog, previews and downloads go
  through the Rust side. Release builds only contact `www.googleapis.com` and
  `fonts.gstatic.com`, over HTTPS.
- **Downloads are atomic.** Every file of a family is fetched and checked to be a
  real font before any of it replaces what's on disk, so a failed download or
  update leaves nothing half-installed.
- **The catalog is cached for a day**, and the cached copy is used, with a note,
  when Google can't be reached.

### Searching, filtering and sorting

My library and Google Fonts share one search, filter and sort system.

**Search** matches family names and tags (and style names in the library;
category and scripts in Google Fonts). Every word must match somewhere.
`"small caps"` matches the phrase, and `-display` leaves out anything that
mentions it. When nothing matches, close names are shown instead, so
`montsrat` finds Montserrat, with a note saying so.

**Filters** sit in a row under the sample text, one chip each:

| Library | Google Fonts |
|---|---|
| Status (active, partly active, inactive, installed by the system), Traits (variable, static, has italics, monospaced), Weight, Width (condensed, normal, expanded), Styles, Tags, Format | Category, Status (not downloaded, in your library, active, update available), Traits, Weight, Styles, Scripts, Axes, Google's tags |

- Each chip opens a panel. Lists show how many fonts each choice would leave
  with the other filters applied, so a choice that empties the list is easy to
  spot. Long lists (tags, scripts, axes) can be searched, and lists where a font
  has several values can match **any** or **all** of the picked ones.
- Weight and Styles are ranges: two handles, typed numbers, quick picks such as
  "Bold to Black", and a bar per step showing how many fonts reach it. A
  variable family counts for every weight its axis covers.
- A set chip reads as what it's set to ("Weight 700 Bold or more"), with a ×
  to remove it. **Clear all** appears once two are set. The row says "3 of 12
  families" while filters narrow it.
- Filters and order are remembered per view between launches. Search text
  isn't, so an old search never hides fonts.

**Sort** is the last control in the row. The library sorts by name, date
modified, number of styles, weight range or activation; Google Fonts by
popularity, name, last updated, number of styles or number of scripts. Each
order starts in its natural direction (newest first, A to Z), and the arrow
beside it reverses it. While a search is typed, the order switches to **Best
match** (whole name, then the start of a name, then a word in it), without
changing the saved order; equal matches in Google Fonts fall back to
popularity.

### Variable fonts

A variable font is one file holding a whole range of styles along one or more
axes: weight, width, optical size, slant, or the designer's own. The app reads
each axis from the font (its range, its default and its name) and works with
it in these places:

- **In the list and grid,** every variable font is shown at its default
  instance, with every axis pinned to its default. The browser would
  otherwise guess from the CSS weight, and a font whose default is Thin
  would show as Regular.
- **In the inspector,** the Specimen tab has a slider for each axis the font
  doesn't hide. The preview follows as you drag, and so do the Waterfall and
  Characters tabs. The reset button puts every axis back to its default.
- **Typing a value:** the number beside each slider can be clicked and typed
  into, as can every other slider's number. Enter applies it and Escape
  cancels. Up and Down step through the range, ten steps with Shift. Values
  outside the font's range are brought into it.
- **Activating:** activating a variable font installs the whole file, so apps
  get every named instance and the full range of each axis.

Axis positions in the inspector are for looking only. They aren't saved, and
the style viewer and Compare show each variable font at its default.

### OpenType features

The inspector's **Features** tab lists every OpenType feature the font
offers, grouped:

- **Ligatures:** standard, contextual, discretionary, historical.
- **Figures:** lining and old-style, proportional and tabular, slashed zero,
  fractions.
- **Position:** superscript, subscript, scientific inferiors, ordinals.
- **Capitals:** small and petite capitals, case-sensitive forms, capital
  spacing, titling.
- **Alternates:** swashes, stylistic alternates, historical forms,
  ornaments.
- **Stylistic sets and character variants,** under the names the designer
  gave them, such as Inter's "Open digits" or "Single-story a".
- **Spacing:** kerning.
- **Other:** any tag the app doesn't recognise, by its tag.

Shaping features that are always on for the scripts that need them (`ccmp`,
`locl`, `mark` and the like) aren't listed.

- **Switching:** each feature is a toggle. Ligatures, contextual alternates
  and kerning start on, as they do in browsers and most apps, so switching
  them off is a change too. Turning on a feature that can't share with
  another turns the other off: old-style turns off lining, tabular turns
  off proportional, and only one of superscript, subscript, ordinals and the
  rest is on at a time.
- **Previewing:** point at a feature, or tab to it, and the preview at the top
  shows it off and on, in text chosen to show it: "1/2 3/4" for fractions,
  "H2O" for subscript, "AVATAR" for kerning. Stylistic sets and variants use
  the preview text you've typed, or a line with the letters and every figure.
  A character variant uses the characters it changes, when the font says
  which.
- **Everywhere in the inspector:** what's switched on applies to the
  Specimen, Waterfall and Characters tabs too. The tab's title counts your
  changes, and the reset button undoes them. They're cleared when you open
  another font, as axis positions are.

### Outlines & metrics

The panel icon beside the sidebar's, at the bottom left (**Ctrl/⌘+J**),
opens a panel docked under the font list, as Zed docks its terminal. It
shows the font open in the inspector, or the last one opened, drawn from its
actual outlines rather than as text.

- **Metrics:** the ascender, cap height, x-height, baseline and descender
  are drawn as lines across the glyphs and labelled with their values in
  font units. A side list gives units per em and the line gap too. If the
  font doesn't state its cap height and x-height, they're measured from H
  and x, and the panel says so.
- **Outlines:** each glyph is drawn as an outline, or filled with **Fill**.
  **Points** marks on-curve points as squares and control points as circles,
  with the handles that join them. A dashed box shows each glyph's advance
  width, so spacing is visible.
- **A glyph's numbers:** click a glyph for its code point, glyph name,
  advance, left and right side bearings, bounds, contours and point counts.
- **Size:** the drawing fits the panel's height until you set a size with the
  slider or by typing pixels per em; **Fit** goes back. Wide words scroll
  sideways. Drag the panel's top edge to make it taller; its height is
  remembered.
- **Smoothing:** on, curves are anti-aliased at sub-pixel precision. Off,
  edges snap to whole pixels. **As rendered** sets the same text at 11, 14,
  18 and 28 px. Its smoothing follows the toggle on macOS; elsewhere the
  system decides, so the outline drawing is the reliable comparison.
- **What you type:** up to 32 characters, drawn one glyph per character
  through the font's character map. No shaping happens, so ligatures and
  OpenType features don't apply. Variable fonts are drawn at their default
  instance. A character the font lacks shows as "no glyph".

### Waterfall

The inspector's **Waterfall** tab sets one line at sixteen sizes, from 72 px down
to 9 px, in the face you're looking at (and at the axis positions set in its
Specimen tab). Lines can run off the edge, as in a classic waterfall, or wrap so
every word shows at large sizes.

### Collections

A collection is a named group of styles: the faces you use for one client or
project, say. Collections are listed in the sidebar.

- **Adding fonts.** The bookmark button on a family's row or card adds every
  style of the family; the inspector's **Collections** button adds just the
  style it's showing. Both show which collections already hold them.
- **Viewing.** Choosing a collection lists only its styles, still grouped by
  family. The family's on/off toggle then acts on those styles alone.
- **Switching on and off.** **Activate all** and **Deactivate all** switch every
  style in the collection.
- **What survives.** Deleting a collection never deletes or deactivates its
  fonts. A style whose file goes missing, on a drive that's unplugged, for
  example, drops out of the list and comes back when the file does.

### Right-click menu

Right-click any family, in the list or the grid, for what you can do with it:
open it in the inspector, show its styles, select it, activate or deactivate
it, add it to collections or tag it (both stay open so you can tick several),
compare it with another of its styles, try it in the style viewer, copy its
name, or show its file in a folder. Inside a collection there's also
"Remove from" that collection. The menu key, or Shift+F10, opens it for the
focused row. Long-press opens it on touch screens.

### Compare

**Compare**, under Tools (or the columns button in the inspector, or
"Compare two styles" in the palette), sets two styles on the same text, so
differences show at a glance. Each side has its own font and style picker.
Swap exchanges them, and opening Compare from the inspector puts the style
you're looking at on the left.

- **Side by side:** your text, at a size you choose, then capitals,
  lowercase, figures and punctuation, and running text. Each row lines up
  across the two styles. Single lines stay on one line rather than wrapping,
  so they stay comparable when one font is wider.
- **Overlay:** each character gets a cell with both glyphs drawn on a shared
  baseline, the first in ink and the second in the accent colour. Differences
  in x-height, width, contrast and serifs show where the shapes part. Running
  text stays side by side; letter overlays mean nothing at that scale.
- **Details:** a table of both styles marks every property that differs:
  family, style, weight, width, slope, format, variable axes, glyph and
  character counts, units per em, version, PostScript name and whether it's
  active.

The two choices, the text, size and layout are remembered between launches.

### Style viewer

**Style viewer**, under Tools in the sidebar, sets a heading font and a body font
on a sample page: headings 1–4, lead, paragraph, list, quote, callout and
caption. Each style can be switched on or off and assigned either font. Sizes
follow a modular scale, so you can see a pairing across the whole hierarchy at
once. You can also change the base size, ratio, line heights and line length,
and edit the text in place. From the inspector, "Try in style viewer" sends the
face you're looking at straight into the page.

- **Pairings.** **Save pairing…** stores the fonts, scale and styles under a
  name. The toolbar shows which pairing is open and when it has unsaved edits.
  Open, rename and delete pairings from the menu on its name. The sample text is
  shared by all pairings, so every comparison uses the same words. If a saved
  font has since been updated, the pairing uses the same family's closest
  style; if the family is gone, it says so.
- **Compare.** The compare button sets a second page beside the first, in a
  saved pairing of your choice, with the same text. The two pages scroll
  together. Compare an edited pairing with its saved version to see exactly
  what changed.
- **Export.** **Export** copies the pairing as CSS, or saves it as a CSS or JSON
  file. The CSS uses custom properties for the two families and line heights,
  then one rule per style switched on, with sizes in rem and pixels. The JSON
  holds the same values, for design tools and scripts.

### Keeping the index current

- **Automatically**, while **Watch folders for changes** is on (Settings,
  Library; on by default): fonts added to, changed in or removed from any
  library folder, including folders inside it, show up within a few seconds,
  with a note saying what changed. Network drives may not report changes.
- **At launch**, and with **Check for changes**: an incremental scan of every
  folder. Only files whose size or modification time changed are read.
- **Rescan** (shown when hovering a folder): re-reads every file in that folder
  and removes styles whose files are gone. Use it when a folder looks wrong.
- **Rescan a folder…**: the same for any folder inside your library, such as one
  vendor's subfolder. Cleanup stays within the folder you chose.

Moved and duplicate files are handled: a font that moves keeps its identity (and
stays active), and duplicate copies are recognised rather than re-read.

Scans run one at a time: a rescan asked for while the watcher is scanning
waits its turn rather than failing.

### Duplicate files

**Duplicate files**, under Tools (or the count in the sidebar's footer), lists
every font saved more than once with identical bytes. Each copy's path is
shown, and the copy the app uses is marked **In use**. The groups that waste
the most space come first.

- **Removing copies.** Remove one copy, keep one copy of a font and trash the
  rest, or trash every extra copy at once (after a confirmation).
- **Nothing is deleted outright.** Copies go to the Trash or Recycle Bin, so
  they can be restored.
- **Always kept:** copies in system font folders, copies an active style is
  registered from (deactivate first), and the last copy of anything. These
  rules are enforced in the backend, not just the UI.
- **Removing the copy in use** is allowed when another copy exists: the app
  switches to the surviving copy.

### Tags

Tags describe typefaces in your own words: "geometric", "client: Acme",
"display only". They belong to families, unlike collections, which hold
individual styles.

- **Adding and removing.** Add a tag from the inspector: type to pick an
  existing tag or create a new one. Remove a tag with the × on its chip.
- **Finding.** Tags show on list rows and grid cards. Choosing a tag in the
  sidebar lists its families, and search matches tag names as well as family
  names.
- **Managing.** Settings → Tags lists every tag with how many families have
  it, and renames or deletes them. Tags can also be renamed or deleted from
  their menu in the sidebar. Deleting a tag leaves the fonts untouched.
- **Folding.** Click **Tags** in the sidebar to fold the whole section away;
  it stays folded between launches. The tag you're viewing stays in sight
  while folded, and dragging a family onto the folded heading opens it, so
  you can still drop onto a tag.

**Tags from Google Fonts.** Google tags its families by structure and feeling
(Sans → Humanist, Expressive → Calm, Technology → Pixel …), each with a 0–100
strength. Settings → Tags → **Import tags** puts them on the families in your
library that are also in Google Fonts. That includes downloaded families and
copies you installed some other way, matched by family name.

- **Strength:** choose how strong a tag must be to apply: Any, Moderate (50+)
  or Strong (80+).
- **Kept up to date:** once imported, tags are re-applied when you download
  from Google Fonts, when the catalog refreshes, and when your folders change.
- **Where they show:** in the sidebar under a folding "From Google Fonts"
  group, and grouped as Google groups them in Settings.
- **Names:** a name Google uses in two groups is qualified, as in "Humanist
  (Sans)" and "Humanist (Serif)".
- **Your changes win:** renaming an imported tag keeps your name, removing one
  from a family keeps it removed, and tags you add by hand stay.
- **Removing them:** **Remove Google's tags** takes every imported tag off and
  stops importing; your own tags stay.

The tags come from the Developer API's `capability=FAMILY_TAGS`. If the API
ever refuses that capability, the font list still loads, without tags.

### Switching fonts on and off, and selecting them

Checkboxes only ever mean selection. Switching fonts on and off is the circle
beside each family's and each style's name, which also shows its state:

- **Empty circle:** off.
- **Half-filled circle:** some styles of the family are on.
- **Ticked circle:** every style is on.
- **Lock:** a system font, always on.

Click the circle to switch the family or style. On a half-on family it
switches on the rest. Its tooltip says exactly what a click will do.

**Selecting.** Tick families, or individual styles in an expanded family, to
act on them together; Ctrl/⌘+A selects everything shown and Esc clears. A bar
floats at the bottom of the list with the count and these actions:

- **Activate** or **Deactivate** the selected styles. Each button shows a count
  when only some of the selection would change.
- **Collection:** add the selection to a collection.
- **Tag:** tag the selected families.
- **Clear:** clear the selection.

Dragging any selected family carries the whole selection onto a collection or
tag.

### Command palette and status bar

**Ctrl/⌘+Shift+P** opens the command palette, as in Zed. It covers every
action in the app: switching views and layouts, scanning, selection, the
font open in the inspector, collections, tags and themes. Type to fuzzy-match
(letters in order, word starts count most); the shortcut shows beside each
command, and recently run commands come first. Typing a family's name jumps
to that family.

The **status bar** along the bottom shows what the app is doing (scan
progress, or whether folders are watched) and the counts: styles, active,
duplicates, each a link to its view. It also holds the palette and Settings,
and a toggle for each pane: the sidebar at the bottom left (**Ctrl/⌘+B**),
the outlines panel beside it (**Ctrl/⌘+J**), and the inspector at the bottom
right (**Ctrl/⌘+Alt+B**).

### Widening the inspector

The inspector's toggle reopens the font you last had open, even after a
restart. While the inspector is open, an arrow beside its toggle widens it
over most of the window for a close look at one font. The font list shrinks
to a strip but stays in view, and the inspector's specimen grows to use the
room.

- **Strip width:** drag the divider between the strip and the inspector to
  size the strip. Its width is remembered. Double-click the divider to reset
  it; it also responds to the arrow keys when focused.
- **Getting back:** click anywhere in the strip to restore the usual widths.
  That click does nothing else. The arrow, now pointing right, does the same,
  and so does Escape; a second Escape closes the inspector.
- **Small windows:** where the inspector floats over the list, widening it
  still leaves the strip showing. Hide the sidebar to leave it more room.

### Settings

Settings opens in its own window over the library (the gear in the status bar,
or **Ctrl/⌘+,**), laid out like Zed's: pages on the left (Appearance, Library,
Google Fonts, Tags, About), settings as rows on the right. The search box
finds any setting and jumps to it. Esc or × closes it.

### Motion

Settings → Appearance → **Motion** sets how the app animates:

- **None:** nothing animates; everything changes at once.
- **Snappy:** quick. The default.
- **Graceful:** slower and softer.

It covers the following:
- **Panes:** the sidebar, inspector and outlines panel slide in and out from
  their edges, and the inspector glides when widened or restored.
- **Fonts:** a font fades in as it loads. One already loaded appears at once,
  so scrolling back through the list never flickers.
- **Command palette:** it drops into place when opened and lifts away when
  closed.
- **Tabs:** a tab's content eases in when you choose it.

Hover feedback and menus follow the same setting. The palette also has
commands for all three. Until you choose, the app follows your computer: if
it asks for reduced motion, Motion is None.

### Themes

As in Zed, you choose a mode (System, Light or Dark) and a theme for each of
light and dark.

- **Built in:** the app's own palette, plus the themes that ship with Zed: One,
  Ayu and Gruvbox (MIT-licensed; see `src/lib/themes/LICENSES.md`).
- **Importing:** Settings → Appearance → **Import theme…** takes any Zed
  theme `.json` file, such as one of the hundreds made for Zed. Imported
  themes are kept in the app's data folder and can be removed there.
- **Readable by design:** a theme's colours are mapped onto the app's tokens,
  then checked. Any text colour below WCAG AA contrast is nudged until it
  passes, and Settings lists what was adjusted. Zed themes are designed for
  code, not for this app's selections and badges, so most need a nudge or
  two.
- **The palette:** has a "Use …" command for every theme.

### Drag and drop

Drag any family, from a list row or a grid card, onto a collection or tag in
the sidebar. A drop adds every style of the family to the collection, or tags
the family. The target highlights while you're over it, and Escape cancels.
A plain click still opens the inspector; a drag starts only once the pointer
has moved a few pixels.

## What's verified, and how

Everything below was run, not just written. Linux is where it ran.

| Area | Status |
|---|---|
| Tests | 81 pass, including 4 against every installed font and 2 against a mock Google Fonts server. Clippy clean. `npm run test:viewer` checks the pairing model and CSS/JSON export; `npm run test:features` the feature catalogue; `npm run test:query` search syntax, filters with counts and sorting |
| TTC extraction | All 83 faces from 15 real collections (incl. 20 MB Noto CJK, CFF) pass fontTools with strict checksums and match the source face's glyphs, cmap and names |
| Frontend | `svelte-check`: 0 errors, 0 warnings. 31 KB gzipped |
| Large libraries | Measured in the running app against 10,000+ fonts; see below |
| Google Fonts | Tested against a local mock of the API serving real font files: invalid key, disabled API, valid key, browsing, download, update, Google unreachable, key removal. **Not yet tested against Google's live API** (unreachable from the build sandbox) |
| Key storage | File fallback tested (owner-only permissions confirmed). **The OS keychain path is untested**: the build machine had no keychain daemon |
| Lucide icons | Every icon name in the app type-checks against the map in `Icon.svelte`, so none can be missing. Candidates were compared on a contact sheet at 16 and 32 px. In the real app: sidebar, rows, status bar, inspector, right-click menu and Settings. Changed after seeing it: Lucide's `contrast` read as a "D" at 16 px, so "some styles on" is Lucide's circle with its right half filled |
| Search, filters, sorting | Unit tests (`test:query`) for search syntax, typo fallback, every facet, counts with other filters applied, any/all matching and each order. In a browser harness with real font files: each library and Google filter panel, weight bars lining up with the handles, sorting, typo search, `-` exclusion, Google's grid and list with typed samples loading full fonts, the closer look, and Activate sending one download-and-activate request. **Not yet run in the Tauri app**, and download-and-activate is untested against Google's live API |
| Motion | Harness: each mode writes its timings, and the rail is caught mid-slide (margin −147px at 60ms of Snappy). The inspector slides in, a tab panel runs its entrance, and the palette animates open and stays mounted while it animates closed. Under None, panes and the palette change instantly. The choice survives a theme change and a reload. Real app: the setting in Settings, and the sidebar sliding open frame by frame. Found and fixed: applying a theme cleared every custom property on `<html>`, motion's timings included |
| Outlines & metrics | Rust: path, point and handle building tested on a hand-made contour; on installed fonts, heights in order, H's top at the cap height, and nothing for a missing character. Against fontTools on Inter: H, g and a match on advance, side bearings, bounds and contours, as do the stated x-height and cap height. Real app: Ctrl+J and the status-bar icon; with and without the inspector, where the last font stays; Fill, Points and Smoothing; dragging the panel taller with Fit following; a typed size; selecting a glyph. Found and fixed: the drawing widening the list's column, the header pushing out the close button in narrow panels, and the slider rounding a fitted size and turning Fit off |
| OpenType features | Rust, against fontTools on Inter 4: the feature tags with lookups, stylistic set and variant names (cv05 "Lower-case L with tail"), and every truncation of a hand-built GSUB failing safely. `npm run test:features`: hidden and unknown tags, names, ordering, defaults, exclusive pairs, CSS output. Real app with Inter: real fractions (½) when previewing `frac`, wider tabular figures, the cv03 preview changing exactly the 6, small caps carrying to the Specimen tab, and the tab counting changes. Harness: exclusivity, the reset staying reachable while a feature is focused |
| Pane toggles and widening | Real app: the right toggle reopens the last font after the inspector was closed and the view changed. The arrow widens the inspector and turns to point right. Dragging the divider resizes the strip without selecting text or grabbing the list's scrollbar. A click in the strip only restores the widths. Escape steps back from widened to open to closed, and Ctrl+Alt+B reopens. Harness, at 1000px: the overlay leaves exactly the strip, and a strip click restores it |
| Typed slider values | Harness: click selects the number, typing and Enter apply it, out-of-range values clamp, anything that isn't a number and Escape both keep the old value, Shift+Down steps ten. Real app: typing a Bitter weight of 750 moves the slider and redraws the specimen |
| Folding tags | Real app: folds and stays folded on relaunch, keeps the viewed tag in sight, and opens when a dragged family rests on the heading |
| Right-click menu | Driven in the real app (WebKitGTK): on list rows, grid cards and an installed family; the Collections submenu staying open while ticking; opening with the menu key and returning focus to the row; Compare from the menu; "Remove from" inside a collection |
| Compare | Driven in the real app: opened from the inspector, the second side defaulting to another style of the same family, a different family picked, overlay and side by side, the details table counting differences |
| Palette, status bar, themes | Driven in the real app: palette commands and family jumps, Ctrl+B, status bar links, One Light and One Dark applied, a Zed theme imported through the native file dialog, settings search jumping to a setting. `npm run test:palette` checks the matching; `npm run test:themes` runs every bundled theme through the mapping and the contrast guard |
| Selection and activation | Driven through the real UI: selecting families and single styles, the floating bar activating a selection, a single style switched off leaving its family half on, the same controls in list and grid. Settings window and stacked dialogs checked in the app and in a browser |
| Google tags, active icons | Imported in the running app from the local mock of the API, which serves tags under both of the keys Google's documentation shows. Checked: strength presets re-importing, names qualified by group, the sidebar group, search by tag, and both active icons in list and grid. **Not yet run against Google's live API** |
| Tags, drag and drop, watching, duplicates | Driven through the real UI on Linux. Tags: created in the inspector, shown on rows and cards. Drag and drop: from a list row and a grid card onto a tag and a collection, plus cancelling with Escape. Watching: files copied in and deleted from outside the app were picked up within 4 s; switching it off ignored a change, and switching it on caught up. Duplicates: a spare copy and the copy in use were moved to the real Trash, with the style switching to the surviving copy; an active style's copies were shown as protected |
| Collections, waterfall, pairings | Driven through the real UI on Linux: creating collections from a card and the inspector, the collection view in both layouts, waterfall in both modes, saving, editing and comparing pairings, in light and dark. Rename, delete, name checks, scroll sync and export were checked in a browser against mocked IPC |
| Targeted rescan | Driven through the UI: per-folder rescan, a subfolder via the picker, rename and delete handling, progress, and refusal of folders outside the library |
| End to end on Linux | Driven through the real UI under a virtual display: preview of static, variable and collection faces; family and single-face activation confirmed by `fc-list`; family toggles showing all, some or none; state survives restart and full rescan; glyph map scrolled through 44,810 characters |
| macOS activation | **Not compiled.** No macOS target was available. `list_active` is a deliberate stub (see below) |
| Windows activation | **Not compiled.** Same reason |

To rerun the extraction check yourself:

```sh
cargo run -p font-core --example probe --release -- /usr/share/fonts /tmp/extract
python3 scripts/validate_extraction.py /tmp/extract /usr/share/fonts
```

## Performance with large libraries

Measured in the release build against a generated library of 10,220 font files
(1.5 GB, 3,888 families, with 140 duplicates), cold disk cache, on a 2-core VM
with software rendering. Times are from the running app, not a microbenchmark.

| | Before | After |
|---|---|---|
| First paint | 3,060 ms | 311 ms |
| Toggling a family | 3,209 ms | 53–84 ms |
| Re-render after a change | 442 ms | 18 ms |
| Fonts loaded during a fast fling | every row passed | 27 |
| Duplicate files re-read per scan | all of them | 0 |
| Conflict check | 70 ms | 16 ms |
| Cold scan, 10k files | 1.5 s | 1.5 s |
| Incremental scan, nothing changed | 30 ms | 30 ms |

What made the difference, in order of impact:

1. **Virtualised family list.** Only rows near the viewport exist. Rows are
   measured as they appear, and the scroll position is corrected when a row
   above the viewport turns out taller or shorter than estimated.
2. **No `fc-cache -f` on Linux.** Forcing makes fontconfig sleep ~2 s to outlast
   timestamp granularity. The quick update takes ~10 ms; the cache is then
   checked against the links on disk (~11 ms) and forced only on a mismatch.
   80 rapid add/remove cycles were all picked up correctly without forcing.
3. **Toggles patch local state** instead of reloading every family (megabytes
   at this size). Only the changed families re-render.
4. **A row must stay visible ~120 ms before its font loads**, so flinging past
   thousands of rows costs almost nothing; already-loaded fonts show instantly.
5. **A `files` table** separate from faces, so duplicate copies are known and
   skipped, and moved files are repointed rather than removed.
6. **Scans release the database while reading**, report progress, and a scan
   that changed nothing doesn't reload the list.
7. **Previews of collection faces are extracted in memory.** Only activation
   writes to the disk cache, which is pruned at startup.

One thing that did *not* help: a byte budget for loaded preview fonts. Webview
memory plateaued at ~620 MB scrolling repeatedly through ~80 CJK collection
faces with or without it. It remains as a safeguard.

To reproduce:

```sh
# Backend stages against any font folder
cargo run -p font-core --example bench --release -- /path/to/fonts

# UI timings: build with marks shown in an on-screen overlay
VITE_PERF=1 npm run tauri build
```

**Folder watching** answers every change with the same incremental scan:
events are debounced (an installer copies dozens of files), then sizes and
mtimes are compared. A missed or duplicated event can make the index late,
never wrong, and a quiet 10k-font library costs about 30 ms per change.

### Testing Google Fonts without Google

Debug builds read two variables that point the app at a mock server; release
builds ignore them:

```sh
FONT_MANAGER_GOOGLE_API=http://127.0.0.1:8765/webfonts/v1/webfonts \
FONT_MANAGER_GOOGLE_HOSTS=127.0.0.1 \
npm run tauri dev
```

## Before shipping on macOS and Windows

1. **Try Google Fonts with a real key** once, on any platform: check that the
   catalog loads and a download works. Only the mock has been exercised so far.
2. **Check the key lands in the keychain** on each platform (Settings says where
   it went).
3. **Compile `activate/macos.rs` and `activate/windows.rs`.** Written against the
   documented APIs but never seen by a compiler. Expect small fixes.
4. **Finish `MacActivator::list_active`.** It returns an empty list on purpose.
   Startup reconciliation trusts this list, and a wrong answer silently corrupts
   state. With the stub, the effect is harmless — every active font is
   re-registered at launch (registration is idempotent) — but crash debris in the
   cache is never cleaned up. Walk the `CFArray` from
   `CTFontManagerCopyRegisteredFontDescriptors` and read `kCTFontURLAttribute`.
5. **Confirm `kCTFontManagerScopePersistent == 2`** against `CTFontManager.h`.
6. **Ship macOS unsandboxed and notarised.** Persistent registration is restricted
   in the sandbox.

## Design

The interface follows a small design system documented in
[DESIGN.md](DESIGN.md): colour roles with WCAG AA contrast in light and dark
themes, spacing, type and radius scales, and a component kit built on
[Bits UI](https://bits-ui.com) for accessible menus, selects, sliders, tabs,
tooltips and dialogs.

## How it's put together

```
crates/font-core/      Everything substantive; no Tauri dependency
  sfnt.rs              Extract face N of a .ttc as a standalone font
  parse.rs             Names (typographic → legacy fallback), axes, coverage
  family.rs            Grouping, style order, conflicts, duplicates
  google.rs            Google Fonts client: catalog, downloads, host allowlist
  db.rs                SQLite index: files, faces, activation ledger,
                       collections, tags, saved pairings, duplicate groups
  scan.rs              Parallel, incremental scanning with progress
  activate/            Platform activation behind one trait
src-tauri/src/lib.rs   State, the font:// protocol, IPC commands
src-tauri/src/google_cmds.rs  Settings, key, catalog, downloads, preview proxy
src-tauri/src/secrets.rs      Keychain storage with a private-file fallback
src-tauri/src/library_cmds.rs Collections, tags, pairings, duplicates, exports
src-tauri/src/watch.rs        Folder watching: debounced rescans
src-tauri/src/theme_cmds.rs   Importing and keeping Zed theme files
src/                   Svelte 5 frontend (VirtualList.svelte, fonts.ts loader)
src/lib/viewer/        Style viewer: scale model, pairings, export, page
src/lib/themes/        Zed theme mapping, bundled themes and their licences
src/lib/palette.ts     Command palette model and fuzzy matching
src/lib/ui/            Component kit on Bits UI, icons from Lucide; see DESIGN.md
licenses/LUCIDE.txt    Lucide's ISC licence (and Feather's MIT)
src/app.css            Design tokens
```

### Decisions worth knowing

**Preview never touches the OS.** Font bytes are served over a custom `font://`
protocol and loaded with the `FontFace` API. That one constraint shapes the rest:
a webview can't address face N of a collection, and every OS activation API works
on whole files — so `sfnt.rs` solves preview and face-level activation at once.

**The protocol takes face ids, never paths.** Ids are validated as 64 hex
characters before the index is touched; the path is resolved server-side. Because
an id is derived from the file's content hash, responses are cached as immutable.

**A preview that failed is shown as failed.** A font that doesn't load falls back
to a system font and *looks like a real preview*. Unloaded specimens show a
blank placeholder, never fallback glyphs, and failures are labelled. Preview
family names are synthetic (`pf-<id>`) so an uninstalled font can't collide with
an installed copy of itself.

**Variable fonts render as their file declares.** Browsers map `font-weight:
normal` onto the `wght` axis, which made a font whose default instance is Thin
preview as Regular. Every view pins axes to their declared defaults.

**The ledger records intent; the OS records mechanism.** At startup they are
reconciled: missing registrations are restored, orphans in the cache removed.
Re-scans use `ON CONFLICT DO UPDATE`, never `INSERT OR REPLACE` — replace deletes
the row first, which would cascade and drop the font's activation record.

**Scans don't hold the database.** A scan snapshots known file stats, releases the
lock, then does the slow work, so previews stay responsive throughout.

**Installed fonts can't be toggled, enforced in the backend** — not just hidden in
the UI.

**Collection membership has no foreign key to faces.** A face leaves the index
when its file disappears, and comes back with the same id when it reappears.
A cascading key would silently empty collections every time a drive was
unplugged; instead, queries join against the faces that exist now.

**Exports are written by the backend, to a path the user picks.** The save
dialog opens on the Rust side and only CSS or JSON can be written, so the
webview never names a path on disk.

**Dragging uses pointer events, not HTML drag and drop.** The webview
intercepts native drag and drop on some platforms (Tauri reserves it for file
drops on Windows); pointer events behave the same everywhere, and can be
cancelled with Escape.

**Tags are keyed by family name; collections by style.** A tag describes a
typeface, so it should follow the family through rescans, updates and new
copies. A collection is a working set, so it can hold just the Bold.

## Not built yet

Named instances from `fvar`/`STAT`, and dragging families onto the sidebar
with the keyboard (the bookmark menu and the inspector's tag editor are the
keyboard routes).
