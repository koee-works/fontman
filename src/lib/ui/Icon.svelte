<script lang="ts" module>
  // The app's icons, drawn by Lucide (lucide.dev, ISC licence). Names here
  // say what an icon means in this app ("rescan", "reveal", "duplicates"),
  // and each maps to the Lucide icon that shows that action most directly.
  // So call sites never name Lucide icons, and one meaning always gets one
  // picture. Imports are per icon, so only these end up in the build.
  //
  // Lines are 1.5px at every size (absoluteStrokeWidth), matching the UI
  // text weight; icons appear where they make a control faster to find,
  // never as decoration beside a label that already says it.
  import type { Component } from 'svelte';
  import ALargeSmall from '@lucide/svelte/icons/a-large-small';
  import ArrowLeftFromLine from '@lucide/svelte/icons/arrow-left-from-line';
  import ArrowLeftRight from '@lucide/svelte/icons/arrow-left-right';
  import Bookmark from '@lucide/svelte/icons/bookmark';
  import BookmarkPlus from '@lucide/svelte/icons/bookmark-plus';
  import Check from '@lucide/svelte/icons/check';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import Circle from '@lucide/svelte/icons/circle';
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import CircleMinus from '@lucide/svelte/icons/circle-minus';
  import CircleX from '@lucide/svelte/icons/circle-x';
  import CloudDownload from '@lucide/svelte/icons/cloud-download';
  import Columns2 from '@lucide/svelte/icons/columns-2';
  import Command from '@lucide/svelte/icons/command';
  import Copy from '@lucide/svelte/icons/copy';
  import Download from '@lucide/svelte/icons/download';
  import Ellipsis from '@lucide/svelte/icons/ellipsis';
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import Eye from '@lucide/svelte/icons/eye';
  import EyeOff from '@lucide/svelte/icons/eye-off';
  import ArrowUpDown from '@lucide/svelte/icons/arrow-up-down';
  import ArrowUpNarrowWide from '@lucide/svelte/icons/arrow-up-narrow-wide';
  import ArrowDownWideNarrow from '@lucide/svelte/icons/arrow-down-wide-narrow';
  import ListFilter from '@lucide/svelte/icons/list-filter';
  import LetterB from '@lucide/svelte/icons/letter-b';
  import MoveHorizontal from '@lucide/svelte/icons/move-horizontal';
  import Layers from '@lucide/svelte/icons/layers';
  import FileType from '@lucide/svelte/icons/file-type';
  import Type from '@lucide/svelte/icons/type';
  import Shapes from '@lucide/svelte/icons/shapes';
  import Languages from '@lucide/svelte/icons/languages';
  import FileBraces from '@lucide/svelte/icons/file-braces';
  import FileCode from '@lucide/svelte/icons/file-code';
  import FileInput from '@lucide/svelte/icons/file-input';
  import Files from '@lucide/svelte/icons/files';
  import Folder from '@lucide/svelte/icons/folder';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import FolderPlus from '@lucide/svelte/icons/folder-plus';
  import FolderSearch from '@lucide/svelte/icons/folder-search';
  import FolderSync from '@lucide/svelte/icons/folder-sync';
  import Globe from '@lucide/svelte/icons/globe';
  import Info from '@lucide/svelte/icons/info';
  import LayoutGrid from '@lucide/svelte/icons/layout-grid';
  import LetterH from '@lucide/svelte/icons/letter-h';
  import Library from '@lucide/svelte/icons/library';
  import List from '@lucide/svelte/icons/list';
  import Lock from '@lucide/svelte/icons/lock';
  import Minus from '@lucide/svelte/icons/minus';
  import Monitor from '@lucide/svelte/icons/monitor';
  import MonitorCog from '@lucide/svelte/icons/monitor-cog';
  import Moon from '@lucide/svelte/icons/moon';
  import Newspaper from '@lucide/svelte/icons/newspaper';
  import Palette from '@lucide/svelte/icons/palette';
  import PanelBottom from '@lucide/svelte/icons/panel-bottom';
  import PanelLeft from '@lucide/svelte/icons/panel-left';
  import PanelRight from '@lucide/svelte/icons/panel-right';
  import Pencil from '@lucide/svelte/icons/pencil';
  import Plus from '@lucide/svelte/icons/plus';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Save from '@lucide/svelte/icons/save';
  import Search from '@lucide/svelte/icons/search';
  import Settings from '@lucide/svelte/icons/settings';
  import Share from '@lucide/svelte/icons/share';
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal';
  import Sun from '@lucide/svelte/icons/sun';
  import Tag from '@lucide/svelte/icons/tag';
  import TagPlus from '@lucide/svelte/icons/tag-plus';
  import Tags from '@lucide/svelte/icons/tags';
  import TextAlignStart from '@lucide/svelte/icons/text-align-start';
  import Trash from '@lucide/svelte/icons/trash';
  import TriangleAlert from '@lucide/svelte/icons/triangle-exclamation-point';
  import X from '@lucide/svelte/icons/x';

  /** Name in this app → Lucide icon, and, for pane toggles, the slice filled in. */
  const icons = {
    // Controls
    check: { c: Check },
    minus: { c: Minus },
    'chevron-right': { c: ChevronRight },
    'chevron-down': { c: ChevronDown },
    x: { c: X },
    search: { c: Search },
    plus: { c: Plus },
    more: { c: Ellipsis },
    external: { c: ExternalLink },
    eye: { c: Eye },
    'eye-off': { c: EyeOff },
    sort: { c: ArrowUpDown },
    'sort-asc': { c: ArrowUpNarrowWide },
    'sort-desc': { c: ArrowDownWideNarrow },
    filter: { c: ListFilter },
    // Filter chips: what each one filters by.
    weight: { c: LetterB },
    width: { c: MoveHorizontal },
    styles: { c: Layers },
    format: { c: FileType },
    category: { c: Type },
    traits: { c: Shapes },
    scripts: { c: Languages },
    // Messages
    info: { c: Info },
    warning: { c: TriangleAlert },
    error: { c: CircleX },
    // Activation: an empty circle (off), half filled (some styles on), ticked
    // (all on), a lock (installed by the system, not switchable here). The
    // selection bar's Deactivate is the circle with a minus.
    inactive: { c: Circle },
    // Half on: Lucide's circle with its right half filled, which reads as
    // "half" at 16px where Lucide's own `contrast` (a smaller inner half-disc)
    // reads as a letter D.
    'active-some': { c: Circle, fill: 'M12 2a10 10 0 0 1 0 20z' },
    active: { c: CircleCheck },
    deactivate: { c: CircleMinus },
    lock: { c: Lock },
    // Places
    'all-fonts': { c: ALargeSmall },
    library: { c: Library },
    system: { c: MonitorCog },
    variable: { c: SlidersHorizontal },
    globe: { c: Globe },
    folder: { c: Folder },
    duplicates: { c: Files },
    'style-viewer': { c: Newspaper },
    compare: { c: Columns2 },
    // Folders
    'folder-plus': { c: FolderPlus },
    rescan: { c: FolderSync },
    'rescan-pick': { c: FolderSearch },
    reveal: { c: FolderOpen },
    // Collections and tags
    bookmark: { c: Bookmark },
    'bookmark-plus': { c: BookmarkPlus },
    tag: { c: Tag },
    tags: { c: Tags },
    'tag-plus': { c: TagPlus },
    // Files in and out
    download: { c: Download },
    'cloud-download': { c: CloudDownload },
    'import-file': { c: FileInput },
    export: { c: Share },
    save: { c: Save },
    'file-css': { c: FileCode },
    'file-json': { c: FileBraces },
    // Editing
    edit: { c: Pencil },
    trash: { c: Trash },
    reset: { c: RotateCcw },
    refresh: { c: RefreshCw },
    copy: { c: Copy },
    swap: { c: ArrowLeftRight },
    heading: { c: LetterH },
    'body-text': { c: TextAlignStart },
    // Views
    list: { c: List },
    grid: { c: LayoutGrid },
    // Appearance
    palette: { c: Palette },
    sun: { c: Sun },
    moon: { c: Moon },
    monitor: { c: Monitor },
    // App
    gear: { c: Settings },
    command: { c: Command },
    // Pane toggles: Lucide's panel outlines; the open state fills the pane's
    // slice (in Lucide's 24-unit grid, inside the 2-unit corner radius).
    sidebar: { c: PanelLeft },
    'sidebar-open': { c: PanelLeft, fill: 'M5 3h4v18H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z' },
    'sidebar-right': { c: PanelRight },
    'sidebar-right-open': { c: PanelRight, fill: 'M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4z' },
    'panel-bottom': { c: PanelBottom },
    'panel-bottom-open': { c: PanelBottom, fill: 'M3 15h18v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z' },
    // The inspector's widen arrow: the pane's edge pushed left. Rotated 180°
    // when widened, it points back.
    expand: { c: ArrowLeftFromLine },
  } satisfies Record<string, { c: Component<Record<string, unknown>>; fill?: string }>;

  export type IconName = keyof typeof icons;
</script>

<script lang="ts">
  interface Props {
    name: IconName;
    size?: number;
    /** Only when the icon is the sole content of a control without its own label. */
    label?: string;
  }
  let { name, size = 16, label }: Props = $props();

  const icon = $derived(icons[name] as { c: Component<Record<string, unknown>>; fill?: string });
</script>

<icon.c
  {size}
  strokeWidth={1.5}
  absoluteStrokeWidth
  class="fm-icon"
  role={label ? 'img' : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : 'true'}
  focusable="false"
>
  {#if icon.fill}<path d={icon.fill} fill="currentColor" stroke="none" />{/if}
</icon.c>

<style>
  :global(.fm-icon) {
    flex: none;
    display: block;
  }
</style>
