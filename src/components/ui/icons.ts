// The single re-export point for iconsax glyphs. No other file imports from
// 'iconsax-reactjs'.
//
// The reason is that a name which does not exist in the package imports as
// `undefined` with no TypeScript error and no build error — the barrel is typed
// too loosely to catch it — and React then throws on render, which shows up as
// a white window. Keeping the list here makes that one place to audit.
//
// Every name below was checked against
// node_modules/iconsax-reactjs/dist/esm/index.js (993 exports). Verify any
// addition the same way. Known traps, all absent from the package: `Music2`,
// `Grid`, `Terminal`, `Ruler2`, `Voice`, `MinusCircle` (the package spells it
// `MinusCirlce`, typo and all), `QuestionMark`, `ThumbUp`, `Minimize`,
// `CupStar` and `Lamp1`.
export {
  // Shell and navigation
  Setting2,
  Settings,
  Sun,
  Moon,
  HamburgerMenu,
  More,
  MoreCircle,
  ArrowDown2,
  ArrowUp2,
  ArrowLeft2,
  ArrowRight2,
  ArrowRight3,
  Maximize3,
  Maximize4,

  // Running a batch.
  //
  // `Flash` and not `Play`: Start Processing kicks off a batch job, it does not
  // play audio, and a media triangle on it reads as a transport control. The
  // bolt reads as "run this". `Play`/`Pause` stay exported for anything that
  // genuinely is a transport.
  Flash,
  FlashCircle,
  Magicpen,
  Convertshape,
  RecordCircle,
  Play,
  Pause,

  // Cancelling and closing
  Stop,
  StopCircle,
  CloseCircle,
  CloseSquare,

  // Queue actions
  Add,
  AddCircle,
  Minus,
  Trash,
  Refresh,
  Refresh2,
  RefreshCircle,
  Import,
  Export,
  ExportSquare,
  Copy,
  CopySuccess,
  Save2,
  Edit,
  Edit2,

  // Files and folders
  Folder,
  Folder2,
  FolderOpen,
  FolderAdd,
  Document,
  DocumentText,
  Clipboard,
  ClipboardText,
  Eye,

  // Status
  TickCircle,
  TickSquare,
  Warning2,
  InfoCircle,
  Information,
  Danger,
  Timer,
  Clock,

  // Onboarding and help
  Teacher,
  Lamp,
  LampOn,
  MessageQuestion,
  Book1,

  // Community templates and the web app
  Global,
  Link,
  Link2,
  Send2,
  ShieldTick,
  Bookmark,
  Crown1,

  // Supporting the project
  Coffee,
  Heart,
  HeartCircle,

  // Audio and measurement
  Sound,
  Music,
  MusicSquare,
  MusicCircle,
  Musicnote,
  VolumeHigh,
  VolumeLow,
  Microphone2,
  Slider,
  Activity,
  Chart,
  Chart2,
  Graph,
  StatusUp,
  TrendUp,
  Ruler,
  Layer,
  Category,
  Element3,
  Element4,
  Cpu,
  Filter,
  Star,
  Star1,
  Flag,
  Tag,

  // Cover art: `Gallery` for a slot that could hold one, `MusicSquare` for a
  // track whose file carries none.
  Gallery,
  Image,
} from 'iconsax-reactjs';

// Not an iconsax glyph — Capstone's own mark.
export { CapstoneMark } from './CapstoneMark';
