/// Generate a 22×22 RGBA tray icon buffer.
///
/// The icon is a two-bar meter:
/// - Top bar    = primary ratio (5-hour / session window)
/// - Bottom bar = secondary ratio (weekly window), rendered as a thinner hairline
///
/// Returns raw RGBA bytes (width = 22, height = 22). The caller should create
/// a `tauri::image::Image` via `Image::new_owned(rgba, 22, 22)` and set
/// `icon_as_template(true)` so macOS auto-inverts it in Dark Mode.
pub fn generate_meter_icon(primary_ratio: f64, secondary_ratio: f64) -> Vec<u8> {
    let primary = primary_ratio.clamp(0.0, 1.0);
    let secondary = secondary_ratio.clamp(0.0, 1.0);

    const SIZE: u32 = 22;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];

    let margin = 3;
    let bar_width = SIZE - margin * 2;
    let top_bar_height = 5;
    let bottom_bar_height = 2;
    let gap = 2;

    let top_left = margin;
    let top_y = margin;
    let primary_fill_width = ((bar_width as f64) * primary).round() as u32;

    // Top bar (primary)
    draw_bar(
        &mut rgba,
        SIZE,
        top_left,
        top_y,
        primary_fill_width,
        top_bar_height,
        [0, 0, 0, 255],
    );

    // Bottom bar (secondary)
    let bottom_y = top_y + top_bar_height + gap;
    let secondary_fill_width = ((bar_width as f64) * secondary).round() as u32;
    draw_bar(
        &mut rgba,
        SIZE,
        top_left,
        bottom_y,
        secondary_fill_width,
        bottom_bar_height,
        [0, 0, 0, 255],
    );

    rgba
}

fn draw_bar(
    rgba: &mut [u8],
    width: u32,
    x: u32,
    y: u32,
    bar_width: u32,
    height: u32,
    color: [u8; 4],
) {
    if bar_width == 0 || height == 0 {
        return;
    }
    for dy in 0..height {
        for dx in 0..bar_width {
            let px = x + dx;
            let py = y + dy;
            if px < width && py < width {
                let idx = ((py * width + px) * 4) as usize;
                rgba[idx..idx + 4].copy_from_slice(&color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::generate_meter_icon;

    #[test]
    fn generates_rgba_icon_for_zero_and_full() {
        let rgba = generate_meter_icon(0.0, 1.0);
        assert_eq!(rgba.len(), 22 * 22 * 4);
        // A pixel inside the bottom bar should be opaque black because secondary=1.0
        // bottom bar: y=10..11, x=3..19
        let idx = (10 * 22 + 18) * 4;
        assert_eq!(rgba[idx..idx + 4], [0, 0, 0, 255]);
    }
}
