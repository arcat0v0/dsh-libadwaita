#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Columns {
    pub sidebar: f64,
    pub center: f64,
    pub rightbar: f64,
}

pub const CENTER_MIN: f64 = 400.0;
pub const SIDEBAR_MIN: f64 = 264.0;
pub const SIDEBAR_MAX: f64 = 420.0;
pub const SIDEBAR_DEFAULT: f64 = 280.0;
pub const SIDEBAR_COLLAPSED: f64 = 56.0;
pub const SIDEBAR_DRAWER_BREAKPOINT: f64 = 768.0;
pub const SIDEBAR_AUTO_COLLAPSE: f64 = 1024.0;
pub const RIGHTBAR_MIN: f64 = 300.0;
pub const RIGHTBAR_MAX_RATIO: f64 = 0.7;
pub const RIGHTBAR_DEFAULT_RATIO: f64 = 0.45;

pub fn sidebar_track(viewport: f64, width_preference: f64, collapsed: bool, drawer: bool) -> f64 {
    if drawer {
        return 0.0;
    }
    if collapsed {
        return if viewport < SIDEBAR_DRAWER_BREAKPOINT {
            0.0
        } else {
            SIDEBAR_COLLAPSED
        };
    }
    width_preference.clamp(SIDEBAR_MIN, SIDEBAR_MAX)
}

pub fn solve_columns(viewport: f64, sidebar: f64, rightbar: f64) -> Columns {
    let sidebar = if viewport < SIDEBAR_DRAWER_BREAKPOINT {
        0.0
    } else if viewport < SIDEBAR_AUTO_COLLAPSE && sidebar >= SIDEBAR_DEFAULT {
        SIDEBAR_COLLAPSED
    } else {
        sidebar.clamp(SIDEBAR_MIN, SIDEBAR_MAX)
    };
    let available = viewport - sidebar - CENTER_MIN;
    let rightbar = if rightbar == 0.0 || available < RIGHTBAR_MIN || sidebar == 0.0 {
        0.0
    } else {
        available.min(rightbar.clamp(RIGHTBAR_MIN, viewport * RIGHTBAR_MAX_RATIO))
    };
    Columns {
        sidebar,
        center: (viewport - sidebar - rightbar).max(0.0),
        rightbar,
    }
}
