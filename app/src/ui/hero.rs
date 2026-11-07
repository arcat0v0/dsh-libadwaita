use crate::ui::brand::fish_logo;
use crate::ui::copy::Copybook;
use gtk4::prelude::*;

pub struct HeroParts {
    pub container: gtk4::Box,
    pub composer_slot: gtk4::Box,
    pub preset_menu: gtk4::MenuButton,
    pub preset_label: gtk4::Label,
}

pub fn build_hero() -> HeroParts {
    let copy = Copybook::for_code("system");
    let container = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .css_classes(["dsh-hero"])
        .vexpand(true)
        .hexpand(true)
        .valign(gtk4::Align::Center)
        .halign(gtk4::Align::Fill)
        .build();
    let stack = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .halign(gtk4::Align::Fill)
        .hexpand(true)
        .css_classes(["dsh-hero-stack"])
        .build();

    let headline = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .build();
    if let Some(mark) = fish_logo(34) {
        headline.append(&mark);
    }
    let title_group = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(7)
        .valign(gtk4::Align::Center)
        .build();
    title_group.append(
        &gtk4::Label::builder()
            .label(copy.hero_headline)
            .css_classes(["dsh-hero-title"])
            .build(),
    );
    title_group.append(
        &gtk4::Label::builder()
            .label(copy.preview)
            .css_classes(["dsh-preview-badge"])
            .build(),
    );
    headline.append(&title_group);
    stack.append(&headline);

    let chips = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(2)
        .halign(gtk4::Align::Start)
        .margin_start(20)
        .build();
    let workspace_menu = gtk4::MenuButton::builder()
        .label(copy.default_workspace)
        .icon_name("pan-down-symbolic")
        .css_classes(["dsh-hero-chip"])
        .build();
    let workspace_content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(4)
        .build();
    workspace_content.append(&gtk4::Image::from_icon_name("folder-symbolic"));
    workspace_content.append(
        &gtk4::Label::builder()
            .label(copy.default_workspace)
            .build(),
    );
    workspace_content.append(&gtk4::Image::from_icon_name("pan-down-symbolic"));
    workspace_menu.set_child(Some(&workspace_content));
    let workspace_items = gtk4::gio::Menu::new();
    workspace_items.append_item(&gtk4::gio::MenuItem::new(
        Some(copy.choose_workspace),
        Some("win.new-workspace"),
    ));
    workspace_menu.set_menu_model(Some(&workspace_items));
    crate::ui::menu::suppress_initial_selection(&workspace_menu);
    chips.append(&workspace_menu);

    let preset_menu = gtk4::MenuButton::builder()
        .label(copy.ptc_mode)
        .icon_name("pan-down-symbolic")
        .tooltip_text("选择新任务使用的 Agent 预设")
        .css_classes(["dsh-hero-chip"])
        .build();
    let preset_content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(4)
        .build();
    preset_content.append(&gtk4::Image::from_icon_name("view-grid-symbolic"));
    let preset_label = gtk4::Label::builder().label(copy.ptc_mode).build();
    preset_content.append(&preset_label);
    preset_content.append(&gtk4::Image::from_icon_name("pan-down-symbolic"));
    preset_menu.set_child(Some(&preset_content));
    let preset_items = gtk4::gio::Menu::new();
    for (label, preset) in [
        (copy.standard_mode, "standard"),
        (copy.ptc_mode, "ptc"),
        (copy.minimal_mode, "minimal"),
        (copy.creator_mode, "cordis"),
    ] {
        let item = gtk4::gio::MenuItem::new(Some(label), None);
        item.set_action_and_target_value(
            Some("win.agent-preset"),
            Some(&preset.to_variant()),
        );
        preset_items.append_item(&item);
    }
    preset_menu.set_menu_model(Some(&preset_items));
    crate::ui::menu::suppress_initial_selection(&preset_menu);
    chips.append(&preset_menu);
    stack.append(&chips);

    let composer_slot = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .hexpand(true)
        .halign(gtk4::Align::Fill)
        .css_classes(["dsh-hero-composer-slot"])
        .build();
    stack.append(&composer_slot);
    let bounded = gtk4::Box::builder()
        .hexpand(true)
        .vexpand(true)
        .build();
    let layout = gtk4::ConstraintLayout::new();
    bounded.set_layout_manager(Some(layout.clone()));
    bounded.append(&stack);
    let strength_required = 1001001000;
    let strength_strong = 1000000000;
    layout.add_constraint(gtk4::Constraint::new(
        Some(&stack),
        gtk4::ConstraintAttribute::CenterX,
        gtk4::ConstraintRelation::Eq,
        Some(&bounded),
        gtk4::ConstraintAttribute::CenterX,
        1.0,
        0.0,
        strength_required,
    ));
    layout.add_constraint(gtk4::Constraint::new(
        Some(&stack),
        gtk4::ConstraintAttribute::CenterY,
        gtk4::ConstraintRelation::Eq,
        Some(&bounded),
        gtk4::ConstraintAttribute::CenterY,
        1.0,
        0.0,
        strength_required,
    ));
    layout.add_constraint(gtk4::Constraint::new(
        Some(&stack),
        gtk4::ConstraintAttribute::Width,
        gtk4::ConstraintRelation::Eq,
        Some(&bounded),
        gtk4::ConstraintAttribute::Width,
        1.0,
        -32.0,
        strength_strong,
    ));
    layout.add_constraint(gtk4::Constraint::new_constant(
        Some(&stack),
        gtk4::ConstraintAttribute::Width,
        gtk4::ConstraintRelation::Le,
        952.0,
        strength_required,
    ));
    layout.add_constraint(gtk4::Constraint::new(
        Some(&stack),
        gtk4::ConstraintAttribute::Width,
        gtk4::ConstraintRelation::Le,
        Some(&bounded),
        gtk4::ConstraintAttribute::Width,
        1.0,
        -32.0,
        strength_required,
    ));
    container.append(&bounded);
    HeroParts {
        container,
        composer_slot,
        preset_menu,
        preset_label,
    }
}
