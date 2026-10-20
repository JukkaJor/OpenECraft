//! Small 3D vector helpers and an orbit camera, shared by the 3D views.

pub type V3 = [f32; 3];

pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

pub fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt().max(1e-6);
    [a[0] / l, a[1] / l, a[2] / l]
}

/// Camera orbiting around `target` (y up).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    pub target: V3,
}

impl Orbit {
    /// Eye position and camera axes (right, up, forward).
    pub fn basis(&self) -> (V3, V3, V3, V3) {
        let eye = [
            self.target[0] + self.dist * self.pitch.cos() * self.yaw.sin(),
            self.target[1] + self.dist * self.pitch.sin(),
            self.target[2] + self.dist * self.pitch.cos() * self.yaw.cos(),
        ];
        let fwd = norm(sub(self.target, eye));
        let right = norm(cross(fwd, [0.0, 1.0, 0.0]));
        let up = cross(right, fwd);
        (eye, right, up, fwd)
    }
}

#[cfg(test)]
mod tests;
