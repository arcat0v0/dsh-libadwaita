#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollFollow {
    following: bool,
}

pub const FOLLOW_THRESHOLD: f64 = 32.0;

impl ScrollFollow {
    pub fn new(following: bool) -> Self {
        Self { following }
    }

    pub fn following(&self) -> bool {
        self.following
    }

    pub fn jump(&mut self) {
        self.following = true;
    }

    pub fn pause(&mut self) {
        self.following = false;
    }

    pub fn sample(&mut self, value: f64, upper: f64, page: f64) -> bool {
        self.following = near_bottom(value, upper, page);
        self.following
    }
}

impl Default for ScrollFollow {
    fn default() -> Self {
        Self::new(true)
    }
}

pub fn near_bottom(value: f64, upper: f64, page: f64) -> bool {
    let floor = (upper - page).max(0.0);
    value >= floor - FOLLOW_THRESHOLD
}
