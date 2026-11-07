pub fn theme_tokens(dark: bool) -> &'static str {
    if dark {
        r#"
  --dsh-bg-base: #151517;
  --dsh-sidebar-fill: #1b1b1c;
  --dsh-composer-fill: #2c2c2e;
  --dsh-selector-fill: #353638;
  --dsh-elevated-fill: #43454a;
  --dsh-label-primary: #f9fafb;
  --dsh-label-secondary: #cfd3d6;
  --dsh-label-tertiary: #adb2b8;
  --dsh-label-caption: #81858c;
  --dsh-hover-fill: rgba(255,255,255,0.08);
  --dsh-active-fill: rgba(255,255,255,0.14);
  --dsh-border-l2: rgba(255,255,255,0.12);
  --dsh-border-l3: rgba(255,255,255,0.16);
  --dsh-accent-blue: #7aaaff;
  --dsh-accent-blue-hover: #4176e6;
  --dsh-preview-badge-fill: #34415b;
  --dsh-brand-ink: #1b1b1c;
"#
    } else {
        r#"
  --dsh-bg-base: #ffffff;
  --dsh-sidebar-fill: #f5f6f7;
  --dsh-composer-fill: #ffffff;
  --dsh-selector-fill: #e9ecf2;
  --dsh-elevated-fill: #ffffff;
  --dsh-label-primary: #0f1115;
  --dsh-label-secondary: #61666b;
  --dsh-label-tertiary: #81858c;
  --dsh-label-caption: #81858c;
  --dsh-hover-fill: rgba(0,0,0,0.04);
  --dsh-active-fill: rgba(0,0,0,0.1);
  --dsh-border-l2: rgba(0,0,0,0.1);
  --dsh-border-l3: rgba(0,0,0,0.12);
  --dsh-accent-blue: #4176e6;
  --dsh-accent-blue-hover: #65a9fe;
  --dsh-preview-badge-fill: #e4edfd;
  --dsh-brand-ink: #0f1115;
"#
    }
}

pub fn shell_css(dark: bool) -> String {
    format!(
        r#"
:root {{
  --dsh-sidebar-width: 280px;
  --dsh-composer-max-width: 952px;
  --dsh-radius-sm: 8px;
  --dsh-radius-md: 12px;
  --dsh-radius-lg: 16px;
  --dsh-radius-panel: 28px;
  --dsh-transition: 200ms cubic-bezier(0.4, 0, 0.2, 1);
{tokens}
}}

window, .background, .dsh-window {{
  background-color: var(--dsh-bg-base);
  color: var(--dsh-label-primary);
  font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
}}

headerbar, .dsh-header-bar {{
  background: var(--dsh-bg-base);
  color: var(--dsh-label-primary);
  border-bottom: none;
  box-shadow: none;
  min-height: 32px;
  padding: 0 6px;
}}

.dsh-header-bar button {{
  min-width: 28px;
  min-height: 28px;
  padding: 0;
  margin: 0 2px;
}}

.dsh-header-bar .dsh-icon-button {{
  border: none;
  border-radius: var(--dsh-radius-sm);
  background: transparent;
  color: var(--dsh-label-secondary);
}}

.dsh-header-bar .dsh-header-title {{
  font-size: 13px;
  font-weight: 500;
}}

.dsh-icon-button {{
  background: transparent;
  color: var(--dsh-label-secondary);
  border: none;
  border-radius: var(--dsh-radius-sm);
  min-width: 28px;
  min-height: 28px;
  padding: 0;
}}
.dsh-icon-button:hover {{
  background: var(--dsh-hover-fill);
  color: var(--dsh-label-primary);
}}
.dsh-icon-button:checked {{
  background: var(--dsh-active-fill);
}}

.dsh-sidebar {{
  min-width: 264px;
  padding: 6px 8px;
  color: var(--window-fg-color);
  font-size: 14px;
}}
.dsh-sidebar-rail {{
  min-width: 56px;
  padding: 18px 10px 6px;
}}
.dsh-sidebar-rail .dsh-rail-button {{
  min-width: 36px;
  min-height: 36px;
  border-radius: var(--dsh-radius-md);
  margin-bottom: 12px;
}}
.dsh-brand-row {{
  min-height: 60px;
  padding: 8px 0 8px 8px;
}}
.dsh-brand-wordmark {{
  margin-left: 5px;
}}
.dsh-collapse-button {{
  min-width: 28px;
  min-height: 28px;
  background: transparent;
  border: none;
  border-radius: var(--dsh-radius-sm);
  color: var(--dsh-label-secondary);
}}
.dsh-collapse-button:hover {{
  background: var(--dsh-hover-fill);
}}
.dsh-new-session {{
  min-height: 38px;
  margin: 0 4px;
  padding: 8px 12px;
  border: 1px solid var(--dsh-border-l3);
  border-radius: var(--dsh-radius-md);
  background: var(--dsh-elevated-fill);
  color: var(--dsh-label-primary);
  font-size: 14px;
  font-weight: 500;
}}
.dsh-new-session:hover {{
  background: var(--dsh-selector-fill);
}}
.dsh-panel-row, .dsh-tree-row {{
  min-height: 32px;
  padding: 5px 9px;
  border-radius: var(--dsh-radius-md);
  background: transparent;
  color: var(--dsh-label-primary);
  border: none;
  font-size: 14px;
  font-weight: 400;
}}
.dsh-panel-row:hover, .dsh-tree-row:hover, .dsh-tree-row:checked {{
  background: var(--dsh-hover-fill);
}}
.dsh-panel-row:checked {{
  background: var(--dsh-active-fill);
}}
.dsh-tree-row.dsh-row-selected {{
  background: var(--dsh-hover-fill);
}}
.dsh-panel-glyph {{
  color: var(--dsh-label-secondary);
  margin-right: 6px;
}}
.dsh-section-header {{
  min-height: 36px;
  color: var(--dsh-label-tertiary);
  padding-left: 12px;
  font-size: 13px;
}}
.dsh-section-title {{
  font-size: 13px;
  color: var(--dsh-label-tertiary);
}}
.dsh-search-entry {{
  min-height: 28px;
  border-radius: var(--dsh-radius-sm);
  background: transparent;
  border: 1px solid transparent;
  color: var(--dsh-label-primary);
  padding: 2px 8px;
}}
.dsh-search-entry:focus {{
  border-color: var(--dsh-border-l3);
  background: var(--dsh-hover-fill);
}}
.dsh-filter-button {{
  background: transparent;
  border: none;
  border-radius: var(--dsh-radius-sm);
  color: var(--dsh-label-tertiary);
  min-height: 28px;
}}
.dsh-filter-button:hover {{
  background: var(--dsh-hover-fill);
}}
.dsh-tree-row .dsh-row-label {{
  color: var(--dsh-label-primary);
  font-size: 14px;
}}
.dsh-tree-row .dsh-row-time {{
  color: var(--dsh-label-tertiary);
  font-size: 10px;
}}
.dsh-sidebar-footer {{
  border-top: 1px solid var(--dsh-border-l2);
  padding-top: 4px;
}}
.dsh-sidebar-footer .dsh-panel-row {{
  color: var(--dsh-label-secondary);
}}

.dsh-conversation {{
  background: var(--dsh-bg-base);
}}
.dsh-header-corner {{
  min-height: 30px;
  padding: 0 20px;
}}
.dsh-composer-shell {{
  background: var(--dsh-bg-base);
}}
.dsh-hero {{
  background: var(--dsh-bg-base);
}}
.dsh-hero-stack {{
  margin: 0 24px;
}}
.dsh-hero-title {{
  font-size: 26px;
  line-height: 32px;
  font-weight: 500;
  color: var(--dsh-label-primary);
}}
.dsh-preview-badge {{
  padding: 1px 7px;
  border: 1px solid var(--dsh-border-l2);
  border-radius: 999px;
  background: var(--dsh-preview-badge-fill);
  color: var(--dsh-label-primary);
  font-size: 12px;
  line-height: 18px;
}}
.dsh-hero-chip {{
  padding: 0;
  border: none;
  background: transparent;
  color: var(--dsh-label-primary);
  font-size: 13px;
  font-weight: 500;
}}
.dsh-hero-chip > button {{
  min-height: 28px;
  padding: 0 8px;
  border: none;
  border-radius: var(--dsh-radius-sm);
  background: transparent;
  color: var(--dsh-label-primary);
}}
.dsh-hero-chip > button:hover {{
  background: var(--dsh-hover-fill);
}}
.dsh-hero-chip > button:checked {{
  background: var(--dsh-active-fill);
}}
.dsh-hero-chip image, .dsh-composer-chip image {{
  -gtk-icon-size: 12px;
}}
.dsh-hero-chip .dsh-chevron {{
  color: var(--dsh-label-caption);
}}

.dsh-composer-card {{
  background: var(--dsh-composer-fill);
  border: 1px solid var(--dsh-border-l2);
  border-radius: 28px;
  padding: 8px 0 0;
  box-shadow: 0 4px 16px rgba(0,0,0,0.03), 0 0 24px rgba(0,0,0,0.03);
}}
.dsh-composer-entry {{
  background: transparent;
  border: none;
  color: var(--dsh-label-primary);
  caret-color: var(--dsh-accent-blue);
  font-size: 14px;
  line-height: 24px;
  padding: 4px 8px 0 14px;
  min-height: 52px;
}}
.dsh-composer-entry text {{
  background: transparent;
  color: var(--dsh-label-primary);
}}
.dsh-composer-entry .placeholder, .dsh-composer-placeholder {{
  color: var(--dsh-label-caption);
}}
.dsh-composer-placeholder {{
  background: transparent;
  font-size: 14px;
}}
.dsh-hero-composer-slot {{
  margin-top: 8px;
}}
.dsh-tree-list {{
  background: transparent;
}}
.dsh-row-attention .dsh-panel-glyph {{
  color: var(--dsh-accent-blue);
}}
.dsh-row-unread .dsh-row-label {{
  font-weight: 600;
}}
.dsh-tree-list row {{
  background: transparent;
  padding: 0;
}}
.dsh-tree-list row:selected {{
  background: transparent;
}}

.dsh-composer-toolbar {{
  padding: 2px 8px 6px;
}}
.dsh-attach-button {{
  min-width: 28px;
  min-height: 28px;
  border: none;
  border-radius: 999px;
  background: var(--dsh-selector-fill);
  color: var(--dsh-label-primary);
}}
.dsh-attach-button:hover {{
  background: var(--dsh-hover-fill);
}}
.dsh-composer-chip {{
  min-height: 28px;
  border: none;
  border-radius: var(--dsh-radius-sm);
  background: transparent;
  color: var(--dsh-label-secondary);
  font-size: 13px;
  font-weight: 500;
  padding: 0 8px;
}}
.dsh-composer-chip:hover {{
  background: var(--dsh-hover-fill);
}}
.dsh-composer-chip label {{
  font-size: 13px;
  font-weight: 500;
  color: var(--dsh-label-secondary);
}}
.dsh-composer-status {{
  color: var(--dsh-label-secondary);
  font-size: 12px;
}}
.dsh-send-button {{
  min-width: 34px;
  min-height: 34px;
  border: none;
  border-radius: 999px;
  background: var(--dsh-accent-blue);
  color: #ffffff;
  box-shadow: 0 0 0 0 transparent;
}}
.dsh-send-button:hover {{
  background: var(--dsh-accent-blue-hover);
}}
.dsh-send-button.dsh-running {{
  background: var(--dsh-selector-fill);
  color: var(--dsh-label-primary);
}}
.dsh-right-dock {{
  background: var(--dsh-bg-base);
  border-left: 1px solid var(--dsh-border-l3);
}}
.dsh-right-dock .dsh-dock-tab {{
  min-height: 30px;
  color: var(--dsh-label-secondary);
}}
.dsh-right-dock .dsh-dock-tab:checked {{
  color: var(--dsh-label-primary);
  border-bottom: 2px solid var(--dsh-accent-blue);
}}
.dsh-right-dock .dsh-dock-entry {{
  background: var(--dsh-composer-fill);
  color: var(--dsh-label-primary);
  border: 1px solid var(--dsh-border-l2);
  border-radius: var(--dsh-radius-sm);
}}
.dsh-popover-menu > contents, popover > contents {{
  background: var(--dsh-composer-fill);
  color: var(--dsh-label-primary);
  border: 1px solid var(--dsh-border-l2);
  border-radius: var(--dsh-radius-md);
}}
.dsh-popover-menu > arrow, popover > arrow {{
  background: var(--dsh-composer-fill);
  border: 1px solid var(--dsh-border-l2);
}}
.dsh-popover-menu, popover.background {{
  background: transparent;
  border: none;
  box-shadow: none;
}}
.dsh-popover-menu button, popover button {{
  color: var(--dsh-label-primary);
}}
.dsh-popover-menu button:hover, popover button:hover {{
  background: var(--dsh-hover-fill);
}}
.dsh-card, .card {{
  background: var(--dsh-composer-fill);
  border: 1px solid var(--dsh-border-l2);
  border-radius: 12px;
}}
.accent-badge {{
  background: var(--dsh-accent-blue);
  color: #ffffff;
  border-radius: 999px;
  padding: 2px 8px;
}}

.dsh-welcome-bubble {{
  color: var(--dsh-label-primary);
}}
"#,
        tokens = theme_tokens(dark)
    )
}
