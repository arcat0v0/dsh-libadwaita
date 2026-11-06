use gtk4::glib::object::IsA;
use gtk4::prelude::*;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerOption {
    pub label: String,
    pub description: String,
    pub value: String,
}

pub fn show_trigger_popup<F: Fn(String) + 'static>(
    anchor: &impl IsA<gtk4::Widget>,
    title: &str,
    options: Vec<TriggerOption>,
    on_select: F,
) -> gtk4::Popover {
    let popover = gtk4::Popover::builder().build();
    let on_select = Rc::new(on_select);

    let container = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .margin_start(6)
        .margin_end(6)
        .margin_top(6)
        .margin_bottom(6)
        .width_request(360)
        .build();

    let title_label = gtk4::Label::builder()
        .label(title)
        .css_classes(["title-4"])
        .halign(gtk4::Align::Start)
        .build();
    container.append(&title_label);

    if options.is_empty() {
        let empty = gtk4::Label::builder()
            .label("No options available")
            .css_classes(["dim-label", "caption"])
            .halign(gtk4::Align::Start)
            .build();
        container.append(&empty);
    } else {
        for option in options {
            let button = gtk4::Button::builder().css_classes(["flat"]).build();
            let row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(2)
                .margin_start(4)
                .margin_end(4)
                .margin_top(2)
                .margin_bottom(2)
                .build();
            let label = gtk4::Label::builder()
                .label(&option.label)
                .halign(gtk4::Align::Start)
                .build();
            let description = gtk4::Label::builder()
                .label(&option.description)
                .css_classes(["dim-label", "caption"])
                .wrap(true)
                .halign(gtk4::Align::Start)
                .build();
            row.append(&label);
            row.append(&description);
            button.set_child(Some(&row));

            let pop_clone = popover.clone();
            let on_select_value = option.value.clone();
            let on_select = on_select.clone();
            button.connect_clicked(move |_| {
                on_select(on_select_value.clone());
                pop_clone.popdown();
            });
            container.append(&button);
        }
    }

    popover.set_child(Some(&container));
    popover.set_parent(anchor);
    popover.popup();
    popover
}
