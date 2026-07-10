# Sexy Terminal Panel Usage Document Design System

## 1. Atmosphere & Identity

The document should feel like a calm Korean fintech product guide: bright, direct, and friendly, with enough precision for developers. The signature is a clean white canvas with one confident blue action color and soft gray product panels that make terminal workflows look approachable instead of noisy.

Reference direction: Toss-inspired product clarity, grounded by the official Toss brand resource center color values for Toss Blue and Toss Gray, and adapted to this repository's terminal-panel content without using Toss logos or assets.

## 2. Color

### Palette

| Role | Token | Light | Dark | Usage |
|------|-------|-------|------|-------|
| Surface/primary | --surface-primary | #FFFFFF | #101215 | Main document background |
| Surface/secondary | --surface-secondary | #F7F9FC | #171B21 | Section bands and quiet panels |
| Surface/elevated | --surface-elevated | #FFFFFF | #202632 | Floating product panels |
| Surface/blue-wash | --surface-blue-wash | #EAF3FF | #102A4F | Callouts and selected states |
| Text/primary | --text-primary | #191F28 | #F8FAFC | Headlines and important text |
| Text/secondary | --text-secondary | #333D4B | #CBD5E1 | Body copy |
| Text/muted | --text-muted | #6B7684 | #94A3B8 | Metadata, captions, secondary labels |
| Text/dim | --text-dim | #8B95A1 | #64748B | Disabled and low-priority text |
| Border/subtle | --border-subtle | #E5E8EB | #2A3441 | Dividers and quiet outlines |
| Border/strong | --border-strong | #D1D6DB | #3B4656 | Focused outlines and data rows |
| Accent/primary | --accent-primary | #0064FF | #4B91FF | Primary CTA, links, focus |
| Accent/hover | --accent-hover | #0050D8 | #75AAFF | Hover state |
| Accent/pressed | --accent-pressed | #003EAA | #9DC2FF | Active state |
| Status/success | --status-success | #00C896 | #37DDB2 | Success |
| Status/warning | --status-warning | #FF9500 | #FFB33D | Warning |
| Status/error | --status-error | #F04452 | #FF7782 | Error and destructive |
| Code/background | --code-background | #101828 | #0B1120 | Code blocks and terminal preview |
| Code/text | --code-text | #EAF0FF | #EAF0FF | Code block text |

### Rules

- Toss Blue is the only brand-like hue; other status colors are reserved for state communication.
- White and cool gray surfaces carry most of the layout. Blue appears on calls to action, selected states, focus rings, and important links.
- Do not use the Toss logo or official assets. This system is inspired by the interaction and color posture, not a brand clone.

## 3. Typography

### Scale

| Level | Size | Weight | Line Height | Tracking | Usage |
|-------|------|--------|-------------|----------|-------|
| Display | 56px | 800 | 1.08 | 0 | First viewport headline |
| H1 | 40px | 800 | 1.15 | 0 | Major page headings |
| H2 | 30px | 800 | 1.2 | 0 | Section headings |
| H3 | 22px | 700 | 1.35 | 0 | Card and step titles |
| Body/lg | 18px | 500 | 1.7 | 0 | Hero and lead copy |
| Body | 16px | 500 | 1.7 | 0 | Default paragraphs |
| Body/sm | 14px | 500 | 1.6 | 0 | Secondary details |
| Caption | 12px | 700 | 1.45 | 0 | Labels and metadata |
| Code | 14px | 500 | 1.7 | 0 | Commands and terminal text |

### Font Stack

- Primary: "Pretendard", "Apple SD Gothic Neo", "Noto Sans KR", system-ui, sans-serif
- Mono: "SFMono-Regular", "JetBrains Mono", "Menlo", "Consolas", monospace

### Rules

- Korean copy must be short and direct. Prefer simple verbs and clear next actions.
- Body text never goes below 14px.
- Letter spacing stays at 0 except browser defaults in monospaced code.

## 4. Spacing & Layout

### Base Unit

All spacing derives from a base of 4px.

| Token | Value | Usage |
|-------|-------|-------|
| --space-1 | 4px | Tight inline gaps |
| --space-2 | 8px | Label and icon gaps |
| --space-3 | 12px | Compact block padding |
| --space-4 | 16px | Standard inline rhythm |
| --space-5 | 20px | Comfortable small sections |
| --space-6 | 24px | Default card and panel padding |
| --space-8 | 32px | Grid gaps |
| --space-10 | 40px | Section internal spacing |
| --space-12 | 48px | Major section rhythm |
| --space-16 | 64px | Desktop section spacing |
| --space-20 | 80px | Hero vertical spacing |

### Grid

- Max content width: 1120px
- Column system: responsive 12-column desktop, single-column mobile, 16px mobile margin
- Breakpoints: mobile 375px, tablet 768px, desktop 1280px

### Rules

- Documentation sections use full-width bands with constrained inner content.
- Repeated cards use 8px to 24px radii depending on semantic weight; large product previews may use 28px by design.
- Fixed-format previews must define stable dimensions with aspect ratios or min heights.

## 5. Components

### Primary Button

- Structure: anchor or button with label and optional small arrow glyph built from CSS text.
- Variants: primary blue, secondary blue wash, ghost.
- Spacing: --space-3 vertical, --space-5 to --space-6 horizontal.
- States: default, hover, active, focus, disabled.
- Accessibility: visible focus ring using --accent-primary; text contrast above WCAG AA.
- Motion: transform translateY on hover and active; 160ms ease-out.

### Section Band

- Structure: section element with constrained `.section-inner`.
- Variants: white, gray, blue wash.
- Spacing: --space-16 desktop, --space-10 mobile.
- States: static.
- Accessibility: semantic headings and landmarks.
- Motion: entry fade is optional and disabled under reduced motion.

### Usage Card

- Structure: article with title, body, and optional command block.
- Variants: default, highlight, compact.
- Spacing: --space-6 padding, --space-4 internal gap.
- States: default, hover, focus-within.
- Accessibility: card does not replace real links or buttons.
- Motion: subtle translateY and border-color transition.

### Command Block

- Structure: pre > code plus optional copy button.
- Variants: single command, multi-line recipe.
- Spacing: --space-4 padding, --space-3 controls.
- States: default, hover, focus, copied, error.
- Accessibility: copy buttons have descriptive labels; code remains selectable.
- Motion: opacity and transform only.

### Product Preview

- Structure: decorative terminal panel with session sidebar and grid.
- Variants: hero preview, compact preview.
- Spacing: --space-4 to --space-6.
- States: static, hover.
- Accessibility: hidden from assistive tech when purely decorative; nearby text explains the workflow.
- Motion: ambient transform on hover only.

### Interactive TUI Panel

- Structure: tmux session with one compact session sidebar and a 2x2 or 3x3 terminal grid.
- Sidebar width: 30 terminal columns. Sidebar copy must fit within that width without relying on large blank space.
- Pane frame: content panes show tmux pane border titles at the top so each slot reads as a panel, not a plain shell.
- Empty state: empty slots use a concise waiting state with the slot number and direct action: open an STP terminal in Cursor, then click a row in the sidebar.
- Focus state: active pane uses the tmux active border style; inactive panes keep a quiet border.
- Accessibility: visible text instructions must name the available mouse and keyboard actions without requiring prior tmux knowledge.
- Motion: none; this is a terminal surface.

## 6. Motion & Interaction

### Timing

| Type | Duration | Easing | Usage |
|------|----------|--------|-------|
| Micro | 120ms | cubic-bezier(0.2, 0.8, 0.2, 1) | Button press and copy feedback |
| Standard | 180ms | cubic-bezier(0.2, 0.8, 0.2, 1) | Hover and focus transitions |
| Emphasis | 420ms | cubic-bezier(0.16, 1, 0.3, 1) | Hero preview entrance |

### Rules

- Animate transform and opacity only.
- Respect `prefers-reduced-motion`.
- Hover effects must never shift surrounding layout.

## 7. Depth & Surface

### Strategy

Mixed: soft shadows for large product panels, tonal shifts for section hierarchy, and subtle borders for command blocks.

| Level | Value | Usage |
|-------|-------|-------|
| Subtle | 0 1px 2px rgba(25, 31, 40, 0.05) | Small cards |
| Default | 0 10px 30px rgba(25, 31, 40, 0.08) | Usage cards and panels |
| Prominent | 0 24px 80px rgba(0, 100, 255, 0.16) | Hero preview |

Borders:

| Type | Value | Usage |
|------|-------|-------|
| Subtle | 1px solid var(--border-subtle) | Cards, dividers |
| Strong | 1px solid var(--border-strong) | Focused command blocks |
