# Design system

How the app looks and behaves, and why. Tokens live in `src/app.css`; components
in `src/lib/ui/`. Interactive components are built on
[Bits UI](https://bits-ui.com), a headless Svelte component library: Bits handles
behaviour, focus and accessibility, and this system supplies the look.

## Principles

**The fonts are the interface.** Every specimen on screen is a different,
opinionated typeface. The chrome uses the platform's own UI font, a restrained
palette and no decoration, so it never competes with what's being judged.

**A specimen is never misrepresented.**
- Specimens always render in true ink. Selection is shown around a specimen (a
  tinted row with an accent edge), never by recolouring the text.
- An unloaded font shows a blank placeholder, never fallback glyphs that could pass
  for the real thing.
- A font that fails to load says so.
- Synthetic bold and italic are disabled.
- Variable fonts render at their declared default instance.

**Colour means something, or it isn't used.** See the roles below.

**Flat, except what floats.** Panels sit flush. Only menus, selects, tooltips
and dialogs have shadows, because they genuinely sit above the page.

**Motion answers the user.** Floating layers fade and scale in by 3% when
opened; controls ease their colour on hover. Nothing moves on its own. All
motion is off under `prefers-reduced-motion`.

## Colour

| Role | Meaning | Tokens |
|---|---|---|
| Cyan | Active: switched on, selected, the primary action | `--cyan`, `--cyan-solid` + `--cyan-on`, `--cyan-soft`, `--selected` |
| Magenta | Two live fonts claim the same name. Nothing else, ever | `--magenta`, `--magenta-soft` |
| Red | Errors and destructive actions | `--red`, `--red-solid` + `--red-on`, `--red-soft` |
| Amber | Warnings | `--amber`, `--amber-soft` |

Each role comes in three forms:
- `--x` for text and icons on the sheet.
- `--x-solid` with `--x-on` for filled surfaces.
- `--x-soft` for tinted backgrounds.

Surfaces are `--lightbox` (app ground and sidebar), `--sheet` (where specimens
sit), `--raised` (floating layers), and `--hover`, `--pressed` and `--selected` for
interaction. Ink is `--ink`, `--muted`, `--line` (control borders) and `--faint`
(separators).

**Contrast.** Every text/background pair meets WCAG AA (4.5:1) in both themes.
Check after any palette change:

```sh
python3 scripts/check-contrast.py
```

**Themes.** As in Zed, there's a mode and a pair of themes:
- **Mode:** System, Light or Dark. System follows `prefers-color-scheme`; an
  explicit choice sets `data-theme` on `<html>`.
- **Themes:** a light theme and a dark theme, used for that mode.
  "Font Manager" is the palette in `app.css`.
- **Zed themes:** any other theme is a Zed theme, bundled (One, Ayu, Gruvbox,
  MIT-licensed, credited in `src/lib/themes/LICENSES.md`) or imported from a
  `.json` file. `themes/zed.ts` maps Zed's colour names onto these tokens:
  - the editor background becomes `--sheet`;
  - panels become `--lightbox`;
  - `text`, `text.muted` and `text.accent` become `--ink`, `--muted` and `--cyan`;
  - borders become `--line` and `--faint`;
  - `error` and `warning` become red and amber.
- **Derived colours:** the ones Zed doesn't name (tinted selection, soft
  backgrounds, text on filled buttons) are derived from the role colours.
  Name-conflict magenta keeps the app's own value, since its meaning is the
  app's.
- **Readability guard:** after mapping, every text/background pair the app
  relies on is held to 4.5:1 (focus rings to 3:1). Colours that fall short
  are nudged toward white or black until they pass, and Settings says which
  ones. `npm run test:themes` runs every bundled theme through this.

## Scales

| | Steps |
|---|---|
| Space (`--s-1`…`--s-8`) | 4, 8, 12, 16, 20, 24, 32, 48 px |
| Type (`--text-1`…`--text-5`) | 12, 13, 15, 18, 22 px. Body is 13 |
| Radius | `--r-1` 4 (checkboxes, chips) · `--r-2` 6 (controls) · `--r-3` 10 (floating panels, cards) · `--r-full` |
| Control height | `--control` 30 · `--control-sm` 24 |
| Motion | `--dur-fast` 100 ms · `--dur` 160 ms · `--ease` |

Radius grows with element size, so nested shapes stay concentric.

## Layout and spacing

- **Status bar.** A 28 px bar runs along the bottom of the window, as in Zed,
  in `--lightbox` with muted 12 px text.
  - **Left:** the sidebar toggle, then what the app is doing: scan progress,
    or whether folders are watched.
  - **Right:** counts, each a link to its view (styles, active, duplicates),
    then the command palette and Settings.
- **Hiding the sidebar.** Ctrl/⌘+B hides the sidebar and the main view takes
  its width; the choice is remembered.
- **Stacking** is a token scale (`--z-*` in `app.css`), bottom to top: the
  selection bar, the inspector over a narrow window, the Settings window,
  dialogs, the command palette, floating layers (menus, selects, popovers),
  tooltips, the drag label. Floating layers sit above every window because
  they open from inside them.
- **Sidebar counts share one right edge.** A row's ⋯ menu sits over its count
  (which steps aside while the menu shows) rather than beside it.
- **The sidebar's middle scrolls; its ends don't.** Search stays at the top,
  and the footer and Settings stay at the bottom, however long the collection
  and tag lists grow.
- **Gutters.** Content areas have a 20 px (`--s-5`) gutter on both sides: list
  rows, grid, inspector, toolbars, callouts.
- **Grid.** Cards are at least 260 px wide, with a 16 px (`--s-4`) gap; columns
  are recomputed from the available width. The last row has a bottom margin
  matching the gutter.
- **Nesting.** The styles inside an expanded family are indented one column:
  each style's checkbox sits under the family's on/off circle, so they read
  as belonging to it. Style names get a fixed 96 px column (truncated, with
  the full name on hover) and previews follow 12 px later, capped at 28 px,
  so the list stays compact.
- **One surface per row.** A row that can be hovered or selected is a single
  shape that carries both states, with its content inset by `--s-2`.
  Selection is a border that's always present but transparent until selected,
  so it never shifts layout or touches the content inside. Never nest a second
  hover shape inside a selected one.
- **Selection in the grid** is the card's border plus a 1 px ring drawn outside
  it, which the grid gap leaves room for.
- **Scroll areas reach the panel edge.** A scrolling region inside a padded panel
  takes a negative right margin and restores the gutter as padding. The
  scrollbar then sits at the edge instead of crowding the text.
- **Specimens in cards** are clamped to two lines, so every card in a row matches
  whatever the sample text.

## Components

| Component | Built on | Use it for |
|---|---|---|
| `Button` | native `<button>` | Actions with a text label. `solid` for the one primary action in a region; `soft` for weighted secondaries; `outline` for neutral rows of choices; `ghost` inside lists and toolbars. Tones: `accent`, `neutral`, `danger` |
| `IconButton` | Bits `Tooltip` | Icon-only actions. Always has a `label`, which becomes both its accessible name and its tooltip |
| `Checkbox` | Bits `Checkbox` | Selection, and only selection. Tri-state: the dash means some styles of a family are selected |
| `ActiveToggle` | Bits `Tooltip` | Switching fonts on and off, and showing whether they are: empty circle (off), half-filled (some styles on), ticked (all on), or a lock for system fonts. A toggle button (`aria-pressed` true, mixed or false) whose tooltip says what a click does. Controlled, so a failed switch never shows an untrue state |
| `Slider` | Bits `Slider` | Preview size, variable axes |
| `OutlinePanel` | | The docked outlines & metrics panel. Draws `glyph_outlines` data as SVG; resized from its top edge |
| `FeaturesPanel` | | The inspector's Features tab. Reads `face_features` and groups through `features.ts`, which holds names, groups, samples, defaults and exclusive pairs |
| `NumberField` | | The number beside every slider. Reads as plain text at rest, shows a hover background, and on click selects its value for typing. Enter and blur commit, clamped to the range; Escape reverts; Up and Down step |
| `Select` | Bits `Select` | Picking one of several options that don't all need to be visible |
| `Segmented` | Bits `ToggleGroup` | Two to four options shown together, such as the theme. Always has a value |
| `Tabs` | Bits `Tabs` | Switching views of the same object (the inspector) |
| `Menu` | Bits `DropdownMenu` | Secondary actions behind ⋯, or behind a labelled button when given `text` (Export). Destructive items go last, after a separator, in red |
| `CollectionMenu` | Bits `DropdownMenu` | Adding styles to collections. A tick means all of them are in that collection, a dash means some. The trigger's bookmark is filled when they're in any |
| `ConfirmDialog` | Bits `AlertDialog` | Anything that can't be undone. The confirm button names the action ("Remove folder"), never "OK" |
| `PromptDialog` | Bits `Dialog` | Naming something: a new collection, a pairing, a rename. Errors (an empty or taken name) appear under the field and the dialog stays open |
| `Callout` | | Status for a region: what happened and what to do next. `bar` spans a region; `box` sits inside a panel. Errors are announced to screen readers |
| `SearchField` | native `<input>` | Search, with a clear button |
| `FontPicker` | Bits `Combobox` | Choosing a family from the whole library by typing; matches preview in their own typeface, capped at 60 so it stays instant |
| `FamilyCard` | | A family in the grid view: two-line specimen, then checkbox, on/off toggle, name and meta |
| `SettingsWindow` | Bits `Dialog` | Settings in their own window, after Zed's. Pages on the left, under a search box that finds any setting and jumps to it with a brief highlight. Each setting is a row: name and description left, control right, hairlines between. Sits below dialogs, so confirmations opened from it stack on top. Brings its own tooltip provider, since it's portalled outside the app's |
| `TagMenu` | Bits `DropdownMenu` | Tagging several families at once, from the selection bar. Ticks and dashes as in `CollectionMenu` |
| `FamilyContextMenu` | | Right-click menu for families, shared by list rows and grid cards; wraps the results area |
| `Compare` | | Two styles on the same text: side by side in aligned rows, or overlaid letter by letter on a shared baseline, with a details table marking differences. State in `compare.svelte.ts` |
| `Waterfall` | | One line of text at sixteen sizes, largest first, in the inspector |
| `TagManager` | | Settings → Tags: every tag with its family count, rename and delete, Google's tags grouped as Google groups them, and the Google import's controls |
| `TagEditor` | Bits `Popover` | A family's tags as removable chips, with a popover to add one. The popover is a combobox: the keyboard stays in the field and arrows move the highlight, and Enter picks it or creates the typed tag |
| `CommandPalette` | Bits `Dialog` | Ctrl/⌘+Shift+P: every action, fuzzy-matched (`palette.ts`), with its shortcut and group, recent ones first; families, collections and tags by name. The keyboard stays in the field (combobox pattern) |
| `Icon` | Lucide | Every icon, by a name that says what it means in this app (`rescan`, `reveal`, `duplicates`), mapped to one Lucide icon, so one meaning always gets one picture. 16 px with 1.5 px lines at any size (`absoluteStrokeWidth`). An entry can add a solid `fill` path on top of the Lucide outline: the open pane slices, and the half-filled circle of `active-some`. Rename is always `edit` (the pencil). Icons make a control faster to recognise; they're never decoration beside a label that already says it |

Bits parts are styled through `fm-` prefixed classes with `:global()` inside the
owning component. That keeps the styles contained, while still reaching floating
content, which Bits portals to `<body>`.

**Never put a native `title` on a Bits trigger.** In WebKitGTK (the Linux
webview), a `title` on a dropdown trigger stops the menu opening on click. Name
icon-only triggers with `aria-label`; for a visible hint, use `IconButton`,
which has a proper tooltip.

## Patterns

- **Destructive actions** confirm first and say what survives: removing a folder
  keeps the files on disk; removing the API key keeps downloaded fonts.
- **Feedback** names the result: "Rescanned big: 10,220 files read",
  "Downloaded Inter to your library".
- **Empty states** say what to do and offer the action.
- **Revealed controls**, such as a folder's ⋯ menu, appear on hover or keyboard
  focus, and are always visible on touch screens.
- **Selection is never activation.** Checkboxes select; the circle beside a
  name switches it on and off. Selecting shows a bar that floats over the
  bottom of the list (so nothing shifts under the pointer) with bulk actions.
  Dragging a selected family drags the whole selection.
- **Installed (system) fonts** show a lock where the on/off toggle would be. They're
  always on and can't be toggled.
- **Two library views**: List (expandable families, every style inline) and Grid
  (one card per family). The toolbar toggle is remembered. In either view, the
  inspector's style picker reaches every style of the family.
- **Revealed controls that carry state stay visible.** A family's bookmark
  button appears on hover, but stays shown while the family is in a
  collection, so membership can be seen at a glance.
- **Collections** hold styles, not families. In a collection's view each family
  is narrowed to its member styles, and its checkbox acts on those alone. The
  header offers Activate all and Deactivate all; deleting a collection says
  plainly that its fonts stay.
- **Active at a glance.** The on/off toggle is the state: cyan `active`
  (ticked circle) when every library style is on, cyan `active-some`
  (half-filled circle) when some are, muted `inactive` (empty circle) when
  none are.
- **Tags** are on families and show as small neutral chips: on a list row after
  the meta, on a card after a tag icon (so a tag can't be mistaken for meta
  like "variable"), and in the inspector, where they can be removed.
- **Dragging** a family shows a label that follows the pointer. It turns cyan
  over a valid target, and the target gets the selected tint with a cyan ring.
  Rows and cards are drag sources; sidebar collections and tags are targets.
  Controls inside a row (checkboxes, menus) never start a drag. Drop targets
  are marked with `data-drop="<kind>:<id>"`.
- **Duplicate files** is a review screen, not a cleanup wizard:
  - **What's shown:** every copy with its path, the folder muted and the file
    name in ink.
  - **Copy labels:** pills mark the copy in use, system copies and active
    copies.
  - **Safety:** a copy that can't be trashed has its Trash button disabled,
    with the reason in its accessible name. The bulk action confirms first and
    says how much space it frees.
- **Icons say what happens, not just where you are**, using Lucide's most
  specific icon for each action:
  - **Adding:** a folder `folder-plus`, a collection `bookmark-plus`, a tag
    `tag-plus`.
  - **Rescanning:** `folder-sync`. Picking a folder to rescan:
    `folder-search`.
  - **Revealing a file:** `folder-open`. Folders themselves stay `folder`.
  - **Files in and out:** importing a theme file `file-input`, importing
    from Google `cloud-download`, exporting `share`, saving CSS
    `file-code`, saving JSON `file-braces`.
  - **Copying:** `copy`, kept apart from the duplicate-files view's `files`.
  - **Activation:** an empty circle, a half-filled circle and a ticked
    circle for off, some and all; `lock` for system fonts; `circle-minus`
    to deactivate a selection.
  - **Places:** all fonts `a-large-small` ("Aa"), the library `library`,
    system fonts `monitor-cog`, the style viewer `newspaper` (a heading
    over text), Compare `columns-2`.
- **Motion** comes from one setting (`lib/motion.svelte.ts`).
  - **Tokens:** it writes the timing tokens on `<html>`: `--dur-fast`
    (hover), `--dur` (menus, palette, tabs), `--dur-pane` (panes),
    `--dur-slow` (fonts fading in) and `--ease`. None sets them all to zero
    and also stops any keyframe outright. Nothing else hard-codes a
    duration.
  - **Panes:** panes slide by a negative margin on their edge (`drawer`).
    Their grid columns are `auto`, so the column shrinks with the pane and
    the list follows. The pane keeps its size, so its content never reflows
    mid-slide. The status bar sits above the bottom panel as it slides
    under.
  - **Palette:** it uses CSS keyframes on `data-state`. Bits keeps a closing
    dialog mounted until its animations finish.
  - **Fonts:** a specimen fades in only when its font had to load
    (`data-fresh`), never when it came from the cache.
- **Pane boundaries** are 2px (`--pane-rule-width`) in `--faint`, a step
  heavier than the 1px hairlines inside panes, so the panes read as separate
  areas without a stronger colour. They cover the sidebar and list, the list
  and inspector, the list and bottom panel, and the sidebar and bottom panel.
  - **Window frame:** an edge that meets only the frame (the list's left
    edge with the sidebar hidden) has no line.
  - **Status bar:** it is a bar, not a pane, and keeps its 1px top rule.
- **Side panes** are toggled from the status bar's corners, as in Zed: the
  left pane's icon at the far left, the right pane's at the far right, the
  bottom panel's beside the left's. Each icon is a window outline with its
  pane's slice marked: drawn as an outline while the pane is closed, the
  slice filled while it's open (`sidebar` / `sidebar-open` and so on). The
  icon carries the state, so these toggles take no pressed background. They
  keep `aria-pressed` and the brighter ink.
- **Widened inspector:**
  - **The arrow:** it appears beside the right-pane toggle only while the
    inspector is open, and rotates 180° to point back once widened.
  - **The strip:** the list keeps a strip of a width you drag to set. Its
    rows ignore the pointer, with no hover, drag or menu, because a click
    there only restores the widths. A capture-phase listener on `main`
    swallows that click before anything in the list sees it.
  - **The divider:** it sits just past the strip's edge, over the
    inspector's margin, so the list's scrollbar stays grabbable. It is an
    ARIA `separator` that takes the arrow keys.
  - **The inspector:** it is a size container, so its display line
    (`10cqi`, 56–144px) grows with its width, while running text keeps a
    68ch measure.
- **The bottom panel** (outlines & metrics) is docked as Zed docks its
  terminal: it sits in its own grid row under the list. The sidebar and
  inspector span both rows, so the panel never runs under them.
  - **Toggle:** its icon sits beside the sidebar's, since both belong to the
    left and center.
  - **Width:** it is `min-width: 0` and a size container. A wide drawing
    scrolls inside it, and the header sheds the font name, then the size
    slider, as it narrows, so its close button always stays in view.
  - **Colours:** metric lines are hairlines in `--line`, with the baseline
    stronger, and labels are muted. Outlines are in ink. On-curve points
    are solid accent squares, control points hollow accent circles, and
    handles muted. The selected glyph's advance box takes `--selected`, like
    any selected row.
- **OpenType features** are toggle chips, with the tag shown muted beside
  the plain name for those who think in tags.
  - **On:** a chip that's on takes the accent tint, like any pressed toggle.
  - **Preview:** pointing at or focusing a chip shows that feature off and
    on in the sticky preview. A feature's effect is usually confined to a
    few characters, so each one carries its own sample. The preview keeps
    one height in both states, so chips never move under the pointer.
  - **Reset:** it lives in the preview heading, which is always present, so
    it stays reachable from the keyboard.
- **Every slider has a typed value.** A slider is for feel; a number is for
  precision, so each slider's value is a `NumberField` rather than a static
  read-out. It stays quiet until hovered, so rows of sliders don't turn into
  a form.
- **Folding sidebar sections:** the heading itself is the toggle, with its
  chevron after the name so it lines up with headings that don't fold. A
  folded section still shows the item being viewed, and opens when a drag
  rests on it (`data-spring` in `drag.ts`).
- **Context menus** are one menu per area, not one per row: rows are
  virtualised, so the menu finds its family by `data-family` on the row or
  card under the pointer. It uses the same `.fm-menu` look as every dropdown.
  The rules:
  - **No browser menu:** right-clicking outside a family shows nothing, not
    even the browser's own menu.
  - **Target ring:** the right-clicked row gets a 1px focus-colour ring while
    the menu is open, so it's clear what the menu acts on.
  - **Order:** groups run open, show and select; activation; organise
    (collections, tags); look closer (compare, style viewer); file (copy
    name, show in folder); then "Remove from" the current collection.
  - **Submenus:** they open level with their trigger. Checkbox submenus stay
    open while ticking.
  - **System families:** activation becomes a disabled "Installed with the
    system" line, rather than vanishing.
  - **Keyboard:** the menu key and Shift+F10 open the menu under the focused
    control, and focus goes back there afterwards.
- **Compare** keeps one colour per side everywhere: a dot of ink marks the
  first style and a dot of accent the second, in the pickers, the table
  header and the overlay. The rules for each layout:
  - **Side by side:** rows are pairs in a two-column grid, so they line up.
    Single-line rows don't wrap; they fade at the edge, like the waterfall.
  - **Overlay:** letter by letter. Both glyphs share one grid area with
    `align-items: baseline`, which aligns their baselines whatever each
    font's ascent.
  - **Details:** differing rows are set in bold with an accent dot, and the
    dot also has a text equivalent for screen readers.
- **Waterfall** is a tab in the inspector, next to Specimen and Characters, so
  it uses the same face and axis positions.
- **Style viewer** (Tools in the sidebar) previews a heading font and a body font
  together on a document page:
  - **Styles:** Heading 1–4, Lead, Paragraph, List, Quote, Callout and Caption,
    each with an on/off toggle and a choice of the heading or body font.
  - **Sizes:** each comes from a modular scale, base × ratio^step, so changing
    the base or ratio re-proportions the whole page.
  - **Spacing:** in `em`, so the vertical rhythm scales with the type.
  - **Line length:** the page column's width, in the body font's own
    characters.
  - **Text:** editable in place, saved as you type, restored by Reset text.

  The inspector's "Try in style viewer" buttons send the open face in as either
  font.

  The toolbar is about the pairing as a whole:
  - **Title.** The pairing's name is the title and opens its menu: open, save
    as new, rename and delete.
  - **Unsaved edits.** "Edited" appears beside the name in muted text, since
    it's a state, not a warning. The save button changes to match: **Save
    pairing…** when nothing is saved, and **Save changes** when edited.
  - **Compare.** Splits the page into two columns. The left page is the
    editable one, and its column says so; the right page is read-only, in a
    saved pairing chosen from its header. The pages scroll together in
    proportion, and narrow their margins to fit.
  - **Export.** Copies or saves CSS or JSON.
- **Keyboard shortcuts:**
  - Ctrl/⌘+F or `/` searches families.
  - Ctrl/⌘+, opens Settings.
  - Ctrl/⌘+Shift+P opens the command palette.
  - Ctrl/⌘+B hides or shows the sidebar.
  - Ctrl/⌘+A selects every style shown.
  - Esc closes whatever is on top: an open menu, select or dialog first, then
    the inspector, then the selection.

  Everything else follows Bits UI's keyboard behaviour: arrow keys in menus,
  selects, sliders and tabs; Esc to close floating layers.

## Adding a component

1. Use a Bits UI primitive if one exists. Don't reimplement focus management,
   keyboard handling or ARIA.
2. Style only with tokens. A new colour is a new role, added to the table above
   and to `scripts/check-contrast.py`.
3. Check it in both themes, with the keyboard alone, and under reduced motion.
