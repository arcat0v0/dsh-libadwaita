use gtk4::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

pub fn suppress_initial_selection(button: &gtk4::MenuButton) {
    let pointer_pressed = Rc::new(Cell::new(false));
    let gesture = gtk4::GestureClick::new();
    let pressed = pointer_pressed.clone();
    gesture.connect_pressed(move |_, _, _, _| pressed.set(true));
    button.add_controller(gesture);

    let weak_button = button.downgrade();
    button.connect_popover_notify(move |button| {
        let Some(popover) = button.popover() else {
            return;
        };
        let pressed = pointer_pressed.clone();
        let weak_button = weak_button.clone();
        popover.connect_show(move |_| {
            if !pressed.replace(false) {
                return;
            }
            let Some(button) = weak_button.upgrade() else {
                return;
            };
            if let Some(root) = button.root() {
                root.set_focus(Some(&button));
            }
        });
    });
}
