pub fn shell_css() -> &'static str {
    r#"
:root {
  --dsh-sidebar-width: 280px;
  --dsh-composer-max-width: 952px;
  --dsh-sidebar-inline-padding: 8px;
  --dsh-sidebar-row-padding: 12px;
  --dsh-sidebar-label-gap: 6px;
  --dsh-sidebar-paint-inset: 3px;
  --dsh-radius-sm: 8px;
  --dsh-radius-md: 12px;
  --dsh-radius-lg: 16px;
  --dsh-radius-panel: 28px;
  --dsh-transition: 200ms cubic-bezier(0.4, 0, 0.2, 1);
  --dsh-hover-fill: color-mix(in srgb, currentColor 10%, transparent);
  --dsh-active-fill: color-mix(in srgb, currentColor 14%, transparent);
  --dsh-surface-window: var(--window-bg-color);
  --dsh-surface-view: var(--view-bg-color);
  --dsh-surface-card: var(--card-bg-color);
  --dsh-surface-popover: var(--popover-bg-color);
  --dsh-surface-headerbar: var(--headerbar-bg-color);
  --dsh-surface-sidebar: var(--sidebar-bg-color);
}

.dsh-window {
  background-color: var(--window-bg-color);
  color: var(--window-fg-color);
}

.dsh-window-chrome {
  padding: 0;
}

.dsh-window-chrome windowcontrols > button {
  min-width: 24px;
  min-height: 24px;
  padding: 0;
  margin: 0 1px;
}

.dsh-chrome-opener {
  margin: 6px 0 0 8px;
}

.dsh-header-bar {
  color: var(--headerbar-fg-color);
  min-height: 32px;
  padding: 0 6px;
}

.dsh-header-bar button {
  min-width: 28px;
  min-height: 28px;
  padding: 0;
  margin: 0 2px;
}

.dsh-header-bar .dsh-icon-button {
  border: none;
  border-radius: var(--dsh-radius-sm);
  background: transparent;
  color: color-mix(in srgb, var(--headerbar-fg-color) 75%, transparent);
}

.dsh-header-bar .dsh-header-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--headerbar-fg-color);
}

.dsh-icon-button {
  background: transparent;
  color: color-mix(in srgb, currentColor 75%, transparent);
  border: none;
  border-radius: var(--dsh-radius-sm);
  min-width: 28px;
  min-height: 28px;
  padding: 0;
}
.dsh-icon-button:hover {
  background: color-mix(in srgb, currentColor 10%, transparent);
  color: var(--window-fg-color);
}
.dsh-icon-button:active, .dsh-icon-button:checked {
  background: color-mix(in srgb, currentColor 14%, transparent);
}
menubutton.dsh-icon-button {
  padding: 0;
  border: none;
  background: transparent;
}
menubutton.dsh-icon-button > button {
  min-width: 28px;
  min-height: 28px;
  padding: 0;
  border: none;
  border-radius: var(--dsh-radius-sm);
  background: transparent;
  color: color-mix(in srgb, currentColor 75%, transparent);
}
menubutton.dsh-icon-button > button:hover {
  background: color-mix(in srgb, currentColor 10%, transparent);
  color: var(--window-fg-color);
}
menubutton.dsh-icon-button > button:active, menubutton.dsh-icon-button > button:checked {
  background: color-mix(in srgb, currentColor 14%, transparent);
}

.dsh-sidebar {
  padding: 6px 8px;
  color: var(--sidebar-fg-color);
  font-size: 14px;
}
.dsh-sidebar-rail {
  min-width: 56px;
  padding: 18px 10px 6px;
}
.dsh-sidebar-rail .dsh-rail-button {
  min-width: 36px;
  min-height: 36px;
  border-radius: var(--dsh-radius-md);
  margin-bottom: 12px;
}
.dsh-brand-row {
  min-height: 60px;
  padding: 8px 0 8px 8px;
}
.dsh-brand-wordmark {
  margin-left: 5px;
}
.dsh-collapse-button {
  min-width: 28px;
  min-height: 28px;
  background: transparent;
  border: none;
  border-radius: var(--dsh-radius-sm);
  color: color-mix(in srgb, var(--sidebar-fg-color) 75%, transparent);
}
.dsh-collapse-button:hover {
  background: color-mix(in srgb, currentColor 10%, transparent);
  color: var(--sidebar-fg-color);
}
.dsh-collapse-button:active {
  background: color-mix(in srgb, currentColor 14%, transparent);
}
.dsh-new-session {
  min-height: 38px;
  margin: 0 4px 10px;
  padding: 8px 12px;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background: var(--card-bg-color);
  color: var(--sidebar-fg-color);
  font-size: 14px;
  font-weight: 500;
}
.dsh-new-session:hover {
  background: color-mix(in srgb, currentColor 10%, var(--card-bg-color));
}
.dsh-new-session:active {
  background: color-mix(in srgb, currentColor 14%, var(--card-bg-color));
}
.dsh-brand-button {
  padding: 0;
  border: none;
  background: transparent;
  min-height: 0;
}
.dsh-brand-button:hover {
  background: transparent;
}
.dsh-brand-button:active {
  background: transparent;
}
.dsh-panel-row {
  min-height: 36px;
  padding: 7px 9px;
  border-radius: 12px;
  background: transparent;
  background-clip: padding-box;
  color: var(--sidebar-fg-color);
  border-top: none;
  border-bottom: none;
  border-left: 3px solid transparent;
  border-right: 3px solid transparent;
  font-size: 14px;
  font-weight: 400;
}
.dsh-panel-row:hover {
  background: color-mix(in srgb, currentColor 10%, transparent);
}
.dsh-panel-row:active {
  background: color-mix(in srgb, currentColor 14%, transparent);
}
.dsh-panel-row.dsh-row-selected {
  background: color-mix(in srgb, currentColor 10%, transparent);
}
.dsh-panel-row.dsh-panel-row-active {
  background: color-mix(in srgb, currentColor 14%, transparent);
}
.dsh-panel-glyph {
  color: color-mix(in srgb, currentColor 75%, transparent);
  margin-right: 6px;
}
.dsh-section-header {
  min-height: 36px;
  color: color-mix(in srgb, var(--sidebar-fg-color) 65%, transparent);
  padding-left: 12px;
  font-size: 13px;
}
.dsh-section-title {
  font-size: 13px;
  color: color-mix(in srgb, var(--sidebar-fg-color) 65%, transparent);
}
.dsh-search-entry {
  min-height: 28px;
  border-radius: var(--dsh-radius-sm);
  background: transparent;
  border: 1px solid transparent;
  color: var(--sidebar-fg-color);
  padding: 2px 8px;
}
.dsh-search-entry:focus {
  border-color: var(--border-color);
  background: color-mix(in srgb, currentColor 10%, transparent);
}
.dsh-filter-button {
  background: transparent;
  border: none;
  border-radius: var(--dsh-radius-sm);
  color: color-mix(in srgb, var(--sidebar-fg-color) 65%, transparent);
  min-height: 28px;
}
.dsh-filter-button:hover {
  background: color-mix(in srgb, currentColor 10%, transparent);
}
.dsh-row-label {
  color: var(--sidebar-fg-color);
  font-size: 14px;
}
.dsh-row-time {
  color: color-mix(in srgb, var(--sidebar-fg-color) 65%, transparent);
  font-size: 10px;
}
.dsh-tree-chevron {
  color: color-mix(in srgb, var(--sidebar-fg-color) 65%, transparent);
}
.dsh-tree-group-header {
  padding: 5px 9px;
  font-size: 13px;
  color: color-mix(in srgb, var(--sidebar-fg-color) 65%, transparent);
}
.dsh-pin-button {
  min-width: 22px;
  min-height: 22px;
  padding: 0;
  border: none;
  border-radius: var(--dsh-radius-sm);
  background: transparent;
  color: color-mix(in srgb, currentColor 75%, transparent);
}
.dsh-pin-button:hover {
  background: color-mix(in srgb, currentColor 10%, transparent);
  color: var(--sidebar-fg-color);
}
.dsh-sidebar-footer {
  padding-top: 4px;
}
.dsh-sidebar-footer .dsh-panel-row {
  color: color-mix(in srgb, var(--sidebar-fg-color) 80%, transparent);
}

.dsh-conversation {
  background: var(--window-bg-color);
  color: var(--window-fg-color);
}
.dsh-header-corner {
  min-height: 30px;
  padding: 0 20px;
}
.dsh-composer-shell {
  background: transparent;
}
.dsh-interaction-card {
  margin: 6px 24px 10px;
  border: 1px solid color-mix(in srgb, var(--warning-color) 35%, var(--border-color));
  background: var(--window-bg-color);
}
.dsh-interaction-strip {
  padding: 8px 16px;
  border-radius: 11px 11px 0 0;
  background: color-mix(in srgb, var(--warning-bg-color) 55%, transparent);
}
.dsh-approval-command {
  padding: 6px 8px;
  border-radius: 6px;
  background: color-mix(in srgb, var(--window-fg-color) 6%, transparent);
}
.dsh-compaction-card {
  padding: 8px 12px;
}
.dsh-subcalls {
  border-left: 2px solid var(--border-color);
  margin: 2px 16px 2px 38px;
  padding-left: 8px;
}
.dsh-scroll-bottom {
  box-shadow: 0 2px 8px var(--shade-color);
}
.dsh-turn-rail {
  padding: 6px 4px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--popover-bg-color) 82%, transparent);
  border: 1px solid var(--border-color);
}
.dsh-turn-mark {
  min-width: 8px;
  min-height: 8px;
  padding: 0;
  border-radius: 999px;
  background-color: var(--border-color);
  border: none;
  box-shadow: none;
}
.dsh-turn-mark:hover {
  background-color: var(--accent-bg-color);
}
.dsh-turn-mark-active {
  background-color: var(--accent-bg-color);
  min-height: 16px;
}
.dsh-fork-boundary {
  padding: 2px 24px;
}
.dsh-hero {
  background: transparent;
}
.dsh-hero-stack {
  margin: 0 24px;
}
.dsh-hero-title {
  font-size: 26px;
  line-height: 32px;
  font-weight: 500;
  color: var(--window-fg-color);
}
.dsh-preview-badge {
  padding: 1px 7px;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  background: color-mix(in srgb, var(--accent-bg-color) 18%, transparent);
  color: var(--window-fg-color);
  font-size: 12px;
  line-height: 18px;
}
.dsh-hero-chip {
  padding: 0;
  border: none;
  background: transparent;
  color: var(--window-fg-color);
  font-size: 13px;
  font-weight: 500;
}
.dsh-hero-chip > button {
  min-height: 28px;
  padding: 0 8px;
  border: none;
  border-radius: var(--dsh-radius-sm);
  background: transparent;
  color: var(--window-fg-color);
}
.dsh-hero-chip > button:hover {
  background: color-mix(in srgb, currentColor 10%, transparent);
}
.dsh-hero-chip > button:active, .dsh-hero-chip > button:checked {
  background: color-mix(in srgb, currentColor 14%, transparent);
}
.dsh-hero-chip image, .dsh-composer-chip image {
  -gtk-icon-size: 12px;
}
.dsh-hero-chip .dsh-chevron {
  color: color-mix(in srgb, var(--window-fg-color) 55%, transparent);
}

.dsh-composer-card {
  background: var(--card-bg-color);
  color: var(--view-fg-color);
  border: 1px solid var(--border-color);
  border-radius: 28px;
  padding: 8px 0 0;
  box-shadow: 0 4px 16px var(--shade-color), 0 0 24px var(--shade-color);
}
.dsh-composer-entry {
  background: transparent;
  border: none;
  color: var(--view-fg-color);
  caret-color: var(--accent-bg-color);
  font-size: 14px;
  line-height: 24px;
  padding: 4px 8px 0 14px;
  min-height: 52px;
}
.dsh-composer-entry text {
  background: transparent;
  color: var(--view-fg-color);
}
.dsh-composer-entry .placeholder, .dsh-composer-placeholder {
  color: color-mix(in srgb, var(--view-fg-color) 55%, transparent);
}
.dsh-composer-placeholder {
  background: transparent;
  font-size: 14px;
}
.dsh-hero-composer-slot {
  margin-top: 8px;
}
.dsh-tree-list {
  background: transparent;
}
.dsh-row-attention .dsh-panel-glyph {
  color: var(--accent-bg-color);
}
.dsh-row-unread .dsh-row-label {
  font-weight: 600;
}
.dsh-tree-list row {
  background: transparent;
  padding: 0;
}
.dsh-tree-list row:selected {
  background: transparent;
}

.dsh-composer-toolbar {
  padding: 2px 8px 6px;
}
.dsh-attach-button {
  min-width: 28px;
  min-height: 28px;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  background: var(--card-bg-color);
  color: var(--view-fg-color);
}
.dsh-attach-button:hover {
  background: color-mix(in srgb, currentColor 10%, transparent);
}
.dsh-attach-button:active {
  background: color-mix(in srgb, currentColor 14%, transparent);
}
.dsh-composer-chip {
  min-height: 28px;
  border: none;
  border-radius: var(--dsh-radius-sm);
  background: transparent;
  color: color-mix(in srgb, var(--view-fg-color) 75%, transparent);
  font-size: 13px;
  font-weight: 500;
  padding: 0 8px;
}
.dsh-composer-chip:hover {
  background: color-mix(in srgb, currentColor 10%, transparent);
}
.dsh-composer-chip:active, .dsh-composer-chip:checked {
  background: color-mix(in srgb, currentColor 14%, transparent);
}
.dsh-composer-chip label {
  font-size: 13px;
  font-weight: 500;
  color: color-mix(in srgb, var(--view-fg-color) 75%, transparent);
}
.dsh-composer-status {
  color: color-mix(in srgb, var(--view-fg-color) 75%, transparent);
  font-size: 12px;
}
.dsh-send-button {
  min-width: 34px;
  min-height: 34px;
  border: none;
  border-radius: 999px;
  background: var(--accent-bg-color);
  color: var(--accent-fg-color);
  box-shadow: 0 0 0 0 transparent;
}
.dsh-send-button:hover {
  background: color-mix(in srgb, var(--accent-bg-color) 86%, currentColor);
}
.dsh-send-button.dsh-running {
  background: color-mix(in srgb, currentColor 10%, transparent);
  color: var(--view-fg-color);
}
.dsh-send-button.dsh-danger {
  background: var(--destructive-bg-color);
  color: var(--accent-fg-color);
}
.dsh-right-dock {
  background: var(--view-bg-color);
  color: var(--view-fg-color);
  border-left: 1px solid var(--border-color);
}
.dsh-right-dock .dsh-dock-tab {
  min-height: 30px;
  color: color-mix(in srgb, var(--view-fg-color) 75%, transparent);
}
.dsh-right-dock .dsh-dock-tab:checked {
  color: var(--view-fg-color);
  border-bottom: 2px solid var(--accent-bg-color);
}
.dsh-right-dock .dsh-dock-entry {
  background: var(--card-bg-color);
  color: var(--view-fg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--dsh-radius-sm);
}
.dsh-card {
  background: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-radius: 12px;
}
.accent-badge {
  background: var(--accent-bg-color);
  color: var(--accent-fg-color);
  border-radius: 999px;
  padding: 2px 8px;
}

.dsh-welcome-bubble {
  color: var(--window-fg-color);
}

.dsh-plugins-tag {
  padding: 1px 8px;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  font-size: 11px;
  color: color-mix(in srgb, var(--window-fg-color) 75%, transparent);
}
.dsh-plugins-tag-accent {
  background: color-mix(in srgb, var(--accent-bg-color) 18%, transparent);
  color: var(--window-fg-color);
}
.dsh-plugins-tag-error {
  background: color-mix(in srgb, var(--destructive-bg-color) 18%, transparent);
}
.dsh-plugins-dot {
  min-width: 10px;
  min-height: 10px;
  border-radius: 999px;
  background: color-mix(in srgb, currentColor 40%, transparent);
}
.dsh-plugins-dot-success {
  background: var(--success-bg-color);
}
.dsh-plugins-dot-ongoing {
  background: var(--warning-bg-color);
}
.dsh-plugins-dot-error {
  background: var(--error-bg-color);
}
.dsh-plugins-skeleton-bar {
  min-height: 12px;
  border-radius: 999px;
  background: color-mix(in srgb, currentColor 12%, transparent);
  animation: dsh-plugins-pulse 1.2s ease-in-out infinite alternate;
}
.dsh-plugins-skeleton-icon {
  min-width: 32px;
  min-height: 32px;
  border-radius: var(--dsh-radius-sm);
  background: color-mix(in srgb, currentColor 12%, transparent);
  animation: dsh-plugins-pulse 1.2s ease-in-out infinite alternate;
}
@keyframes dsh-plugins-pulse {
  from {
    opacity: 1;
  }
  to {
    opacity: 0.35;
  }
}

.dsh-install-terminal {
  font-size: 11px;
  background: color-mix(in srgb, var(--view-bg-color) 92%, var(--card-bg-color));
  border: 1px solid var(--border-color);
  border-radius: var(--dsh-radius-sm);
  padding: 8px 12px;
}
.dsh-install-terminal-header {
  font-weight: 600;
  color: var(--window-fg-color);
  padding-bottom: 4px;
}
.dsh-install-terminal-output {
  color: color-mix(in srgb, var(--window-fg-color) 80%, transparent);
  line-height: 1.4;
}
.dsh-install-terminal-folded {
  font-size: 11px;
  color: color-mix(in srgb, var(--window-fg-color) 60%, transparent);
  padding: 2px 0;
}
.dsh-version-tag {
  padding: 1px 6px;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  font-size: 11px;
}
.dsh-version-tag-warning {
  background: color-mix(in srgb, var(--warning-bg-color) 18%, transparent);
}
.dsh-version-tag-accent {
  background: color-mix(in srgb, var(--accent-bg-color) 18%, transparent);
}
.dsh-version-note {
  font-size: 12px;
  color: color-mix(in srgb, var(--window-fg-color) 65%, transparent);
}
.dsh-version-note-warn {
  color: var(--warning-fg-color);
}

.dsh-schedule-list {
  background: transparent;
}
.dsh-schedule-list row {
  background: transparent;
  padding: 0;
}
.dsh-schedule-list row:selected {
  background: transparent;
}
.dsh-schedule-row {
  min-height: 46px;
  padding: 8px 12px;
  border-radius: var(--dsh-radius-md);
  background: transparent;
  border: 1px solid transparent;
}
.dsh-schedule-row:hover {
  background: color-mix(in srgb, currentColor 8%, transparent);
}
.dsh-schedule-row.dsh-schedule-row-selected {
  background: color-mix(in srgb, var(--accent-bg-color) 12%, transparent);
}
.dsh-schedule-row-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--window-fg-color);
}
.dsh-schedule-row-title.dsh-schedule-row-ended {
  color: color-mix(in srgb, var(--window-fg-color) 55%, transparent);
}
.dsh-schedule-row-summary {
  color: color-mix(in srgb, var(--window-fg-color) 62%, transparent);
  font-size: 12px;
}
.dsh-schedule-caption {
  color: color-mix(in srgb, var(--window-fg-color) 65%, transparent);
  font-size: 12px;
}
.dsh-schedule-notice {
  background: color-mix(in srgb, var(--warning-bg-color) 14%, transparent);
  color: var(--window-fg-color);
  border-radius: var(--dsh-radius-sm);
  padding: 8px 12px;
}
.dsh-schedule-conflict {
  background: color-mix(in srgb, var(--warning-bg-color) 14%, transparent);
  border: 1px solid var(--border-color);
  border-radius: var(--dsh-radius-md);
  padding: 10px 12px;
}
.dsh-schedule-failure {
  color: var(--error-bg-color);
  font-size: 12px;
}
.dsh-schedule-detail {
  background: var(--window-bg-color);
}

.dsh-market-pill {
  min-height: 28px;
  padding: 0 12px;
  border-radius: 999px;
  background: transparent;
  color: color-mix(in srgb, var(--window-fg-color) 70%, transparent);
  font-size: 13px;
}
.dsh-market-pill:hover {
  background: color-mix(in srgb, currentColor 8%, transparent);
}
.dsh-market-pill:checked {
  background: color-mix(in srgb, var(--accent-bg-color) 18%, transparent);
  color: var(--window-fg-color);
}
.dsh-market-notice {
  background: color-mix(in srgb, var(--warning-bg-color) 14%, transparent);
  color: var(--window-fg-color);
  border-radius: var(--dsh-radius-sm);
  padding: 8px 12px;
}
.dsh-plugins-tag-success {
  background: color-mix(in srgb, var(--success-bg-color) 18%, transparent);
}
.dsh-plugins-tag-warning {
  background: color-mix(in srgb, var(--warning-bg-color) 18%, transparent);
}
button.outline {
  border: 1px solid var(--border-color);
  background: transparent;
}
button.outline:hover {
  background: color-mix(in srgb, currentColor 8%, transparent);
}
button.outline:disabled {
  color: color-mix(in srgb, var(--window-fg-color) 40%, transparent);
}
"#
}
