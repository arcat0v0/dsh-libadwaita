use gtk4::prelude::*;
use gtk4::{gdk_pixbuf, gio, glib};
use libadwaita as adw;

pub const FISH_LOGO_SVG: &str = include_str!("../../assets/fish-logo.svg");
pub const BRAND_WORDMARK_SVG: &str = include_str!("../../assets/brand-wordmark.svg");
pub const BRAND_NOTICE: &str = "Official DeepSeek Harness brand artwork, MIT licensed, from deepseek-ai/deepseek-harness fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159. Used without implying endorsement.";

fn load_svg(
    svg: &str,
    width: i32,
    height: i32,
    foreground: &str,
    background: &str,
) -> Option<gtk4::Picture> {
    let themed = svg
        .replace("currentColor", foreground)
        .replace("var(--dsh-brand-ink, #1b1b1c)", background);
    let bytes = glib::Bytes::from_owned(themed.into_bytes());
    let stream = gio::MemoryInputStream::from_bytes(&bytes);
    let pixbuf = gdk_pixbuf::Pixbuf::from_stream_at_scale(
        &stream,
        width * 3,
        height * 3,
        true,
        None::<&gio::Cancellable>,
    )
    .ok()?;
    let picture = gtk4::Picture::new();
    picture.set_paintable(Some(&gtk4::gdk::Texture::for_pixbuf(&pixbuf)));
    picture.set_can_shrink(true);
    picture.set_content_fit(gtk4::ContentFit::Contain);
    picture.set_size_request(width, height);
    Some(picture)
}

pub fn fish_logo(width: i32) -> Option<gtk4::Picture> {
    let (foreground, background) = brand_colors();
    let height = (f64::from(width) * 17.04 / 23.16).round() as i32;
    load_svg(FISH_LOGO_SVG, width, height, foreground, background)
}

pub fn brand_wordmark(width: i32) -> Option<gtk4::Picture> {
    let (foreground, background) = brand_colors();
    let height = (f64::from(width) * 24.0 / 182.0).round() as i32;
    load_svg(BRAND_WORDMARK_SVG, width, height, foreground, background)
}

fn brand_colors() -> (&'static str, &'static str) {
    if adw::StyleManager::default().is_dark() {
        ("#f9fafb", "#1b1b1c")
    } else {
        ("#0f1115", "#ffffff")
    }
}
