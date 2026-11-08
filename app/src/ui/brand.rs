use gtk4::prelude::*;
use gtk4::{gdk_pixbuf, gio, glib};
use libadwaita as adw;

pub const FISH_LOGO_SVG: &str = include_str!("../../assets/fish-logo.svg");
pub const BRAND_WORDMARK_SVG: &str = include_str!("../../assets/brand-wordmark.svg");
pub const BRAND_NOTICE: &str = "Official DeepSeek Harness brand artwork, MIT licensed, from deepseek-ai/deepseek-harness fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159. Used without implying endorsement.";
pub const BRAND_MARK_DARK_FG: &str = "#f9fafb";
pub const BRAND_MARK_DARK_CUTOUT: &str = "#1b1b1c";
pub const BRAND_MARK_LIGHT_FG: &str = "#0f1115";
pub const BRAND_MARK_LIGHT_CUTOUT: &str = "#ffffff";
pub const BRAND_MARK_COLORS: [&str; 4] = [
    BRAND_MARK_DARK_FG,
    BRAND_MARK_DARK_CUTOUT,
    BRAND_MARK_LIGHT_FG,
    BRAND_MARK_LIGHT_CUTOUT,
];

fn render_svg_texture(
    svg: &str,
    width: i32,
    height: i32,
    dark: bool,
) -> Option<gtk4::gdk::Texture> {
    let (foreground, background) = brand_colors(dark);
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
    Some(gtk4::gdk::Texture::for_pixbuf(&pixbuf))
}

fn load_svg(svg: &'static str, width: i32, height: i32) -> Option<gtk4::Picture> {
    let style_manager = adw::StyleManager::default();
    let texture = render_svg_texture(svg, width, height, style_manager.is_dark())?;
    let picture = gtk4::Picture::new();
    picture.set_paintable(Some(&texture));
    picture.set_can_shrink(true);
    picture.set_content_fit(gtk4::ContentFit::Contain);
    picture.set_size_request(width, height);
    let weak_picture = picture.downgrade();
    style_manager.connect_dark_notify(move |manager| {
        if let Some(picture) = weak_picture.upgrade() {
            if let Some(updated) = render_svg_texture(svg, width, height, manager.is_dark()) {
                picture.set_paintable(Some(&updated));
            }
        }
    });
    Some(picture)
}

pub fn fish_logo(width: i32) -> Option<gtk4::Picture> {
    let height = (f64::from(width) * 17.04 / 23.16).round() as i32;
    load_svg(FISH_LOGO_SVG, width, height)
}

pub fn brand_wordmark(width: i32) -> Option<gtk4::Picture> {
    let height = (f64::from(width) * 24.0 / 182.0).round() as i32;
    load_svg(BRAND_WORDMARK_SVG, width, height)
}

pub fn brand_colors(dark: bool) -> (&'static str, &'static str) {
    if dark {
        (BRAND_MARK_DARK_FG, BRAND_MARK_DARK_CUTOUT)
    } else {
        (BRAND_MARK_LIGHT_FG, BRAND_MARK_LIGHT_CUTOUT)
    }
}
