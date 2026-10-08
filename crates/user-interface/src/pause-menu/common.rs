//! Common things for helping

use super::*;

impl Ui {
    pub(super) fn draw_frame(&mut self, r: &Renderer, scene: &mut Scene, rc: [f32; 4], b: f32, c: [u8; 4]) {
        self.text.rounded(r, scene, [rc[0], rc[1], rc[2], rc[1] + b], 0.0, c);
        self.text.rounded(r, scene, [rc[0], rc[3] - b, rc[2], rc[3]], 0.0, c);
        self.text.rounded(r, scene, [rc[0], rc[1], rc[0] + b, rc[3]], 0.0, c);
        self.text.rounded(r, scene, [rc[2] - b, rc[1], rc[2], rc[3]], 0.0, c);
    }
}

pub(super) fn plain(c: [u8; 4]) -> [u8; 4] {
    [c[0], c[1], c[2], 0]
}