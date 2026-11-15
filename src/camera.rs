//! Fixed logical resolutions and integer-scale viewport calculations.

pub const VIRTUAL_WIDTH: u32 = 640;
pub const VIRTUAL_HEIGHT: u32 = 360;
pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 720;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegerViewport {
    pub scale: u32,
    pub width: u32,
    pub height: u32,
    pub offset_x: u32,
    pub offset_y: u32,
}

pub fn integer_viewport(window_width: u32, window_height: u32) -> Option<IntegerViewport> {
    let scale = (window_width / VIRTUAL_WIDTH).min(window_height / VIRTUAL_HEIGHT);
    if scale == 0 {
        return None;
    }
    let width = VIRTUAL_WIDTH * scale;
    let height = VIRTUAL_HEIGHT * scale;
    Some(IntegerViewport {
        scale,
        width,
        height,
        offset_x: (window_width - width) / 2,
        offset_y: (window_height - height) / 2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_1080p_is_exactly_three_times() {
        assert_eq!(integer_viewport(1920, 1080).unwrap().scale, 3);
    }

    #[test]
    fn steam_deck_is_letterboxed_until_a_16_10_mode_is_added() {
        assert_eq!(
            integer_viewport(1280, 800),
            Some(IntegerViewport {
                scale: 2,
                width: 1280,
                height: 720,
                offset_x: 0,
                offset_y: 40,
            })
        );
    }

    #[test]
    fn too_small_window_does_not_produce_fractional_pixels() {
        assert_eq!(integer_viewport(320, 200), None);
    }
}
