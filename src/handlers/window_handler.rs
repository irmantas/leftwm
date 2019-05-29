use super::*;
use crate::display_action::DisplayAction;
use crate::models::XYHW;

/*
 * process a collection of events, and apply them changes to a manager
 * returns true if changes need to be rendered
 */
pub fn created(manager: &mut Manager, a_window: Window) -> bool {
    //don't add the window if the manager already knows about it
    for w in &manager.windows {
        if w.handle == a_window.handle {
            return false;
        }
    }
    let mut window = a_window;
    if let Some(ws) = manager.focused_workspace() {
        window.tags = ws.tags.clone();

        //if dialog, center in workspace
        if window.type_ == WindowType::Dialog {
            window.floating = Some(ws.center_halfed());
            window.set_floating(true);
        }
    } else {
        window.tags = vec![manager.tags[0].clone()]
    }

    if let Some(trans) = &window.transient {
        if window.floating.is_none() {
            window.floating = Some(XYHWBuilder::default().into()); //make sure we have a value to modify
        }
        if let Some(parent) = find_window(manager, &trans) {
            window.floating = Some(calc_center_of_parent(&window, parent));
        }
    }

    window.update_for_theme(&manager.theme_setting);

    //if floating, make sure the window has a floating XYHW
    if window.floating() && window.floating.is_none() {
        window.floating = Some(window.normal);
    }

    manager.windows.push(window.clone());

    //let the DS know we are managing this window
    let act = DisplayAction::AddedWindow(window.handle.clone());
    manager.actions.push_back(act);

    //let the DS know the correct desktop to find this window
    if !window.tags.is_empty() {
        let act = DisplayAction::SetWindowTags(window.handle.clone(), window.tags[0].clone());
        manager.actions.push_back(act);
    }

    //new windows should be on the top of the stack
    let act = DisplayAction::MoveToTop(window.handle.clone());
    manager.actions.push_back(act);

    focus_handler::focus_window(manager, &window, window.x() + 1, window.y() + 1);

    if let Some(cmd) = &manager.theme_setting.on_new_window_cmd {
        use std::process::{Command, Stdio};
        let _ = Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn();
    }

    true
}

/*
 * process a collection of events, and apply them changes to a manager
 * returns true if changes need to be rendered
 */
pub fn destroyed(manager: &mut Manager, handle: &WindowHandle) -> bool {
    let start_size = manager.windows.len();
    manager.windows = manager
        .windows
        .iter()
        .filter(|w| &w.handle != handle)
        .cloned()
        .collect();
    //if we removed the focused window, focus the last window
    focus_handler::focus_last_window_that_exists(manager);

    //make sure the workspaces do not draw on the docks
    update_workspace_avoid_list(manager);

    start_size != manager.windows.len()
}

pub fn changed(manager: &mut Manager, change: WindowChange) -> bool {
    for w in manager.windows.iter_mut() {
        if w.handle == change.handle {
            let changed = change.update(w);
            if w.type_ == WindowType::Dock {
                update_workspace_avoid_list(manager);
                //don't left changes from docks re-render the worker. This will result in an
                //infinite loop. Just be patient a rerender will occur.
                return false;
            }
            return changed;
        }
    }
    false
}

fn calc_center_of_parent(window: &Window, parent: &Window) -> XYHW {
    let mut xyhw = match window.floating {
        Some(f) => f,
        None => XYHWBuilder::default().into(),
    };

    //make sure this window has a real height/width first
    if xyhw.h() == 0 || xyhw.w() == 0 {
        xyhw.set_h(parent.height() / 2);
        xyhw.set_w(parent.width() / 2);
    }

    xyhw.set_x(parent.x() + (parent.width() / 2) - (xyhw.w() / 2));
    xyhw.set_y(parent.y() + (parent.height() / 2) - (xyhw.h() / 2));

    xyhw
}

fn find_window<'w>(manager: &'w Manager, handle: &WindowHandle) -> Option<&'w Window> {
    for win in &manager.windows {
        if &win.handle == handle {
            let r: &Window = win;
            return Some(r);
        }
    }
    None
}

pub fn update_workspace_avoid_list(manager: &mut Manager) {
    let mut avoid = vec![];
    for w in &manager.windows {
        if w.type_ == WindowType::Dock {
            if let Some(f) = w.floating {
                avoid.push(f);
            }
        }
    }
    for w in &mut manager.workspaces {
        w.avoid = avoid.clone();
        w.update_avoided_areas();
    }
}
