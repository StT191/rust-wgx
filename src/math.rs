
pub use std::f32::consts::PI;

pub use glam::{self, *};


// angle conversion
pub fn deg(angle_deg: f32) -> f32 {
    angle_deg * (2.0 * PI) / 360.0
}

pub fn rad(angle_rad: f32) -> f32 {
    angle_rad * 360.0 / (2.0 * PI)
}


// projections
#[derive(Debug, Clone)]
pub struct FovProjection {
    pub fov_deg: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub distance: f32,
    pub projection: Mat4,
    pub translation: Mat4,
}


impl FovProjection {

    pub fn update(&mut self) {
        self.projection = Mat4::perspective_lh(deg(self.fov_deg), self.aspect, self.near, self.far);
        self.translation = Mat4::from_translation([0.0, 0.0, self.distance].into());
    }

    pub fn new(fov_deg: f32, aspect: f32, near: f32, far: f32, distance: f32) -> Self {
        let mut this = Self {
            fov_deg, aspect, near, far, distance,
            projection: Mat4::ZERO, translation: Mat4::ZERO,
        };
        this.update();
        this
    }

    pub fn unit(fov_deg: f32, aspect: f32, unit: f32) -> Self {
        let near = unit / 1.0e3;
        let far = unit * 2.0e3;
        let distance = unit / deg(fov_deg / 2.0).tan();
        Self::new(fov_deg, aspect, near, far, distance)
    }

    pub fn window(fov_deg: f32, width: f32, height: f32) -> Self {
        let unit = f32::max(width, height);
        let near = unit / 1.0e3;
        let far = unit * 2.0e3;
        let distance = height * 0.5 / deg(fov_deg / 2.0).tan();
        Self::new(fov_deg, width/height, near, far, distance)
    }

    pub fn resize_window(&mut self, width: f32, height: f32, update_distances: bool) {

        if update_distances {
            *self = FovProjection::window(self.fov_deg, width, height);
        } else {
            // or aspect only
            self.aspect = width/height;
            self.update();
        };
    }
}


pub fn flat_window_projection(width: f32, height: f32, depth: f32) -> Mat4 {
    Mat4::from_translation([-1.0, 1.0, 0.0].into()) *
    Mat4::from_scale([
        2.0 / width, -2.0 / height, if depth == 0.0 { 0.0 } else { 1.0 / depth },
    ].into())
}



// utilities

#[macro_export]
macro_rules! apply {
    ($matrix:expr, $value:expr) => {
        $matrix = $value * $matrix
    }
}


// in right handed coordinate system
pub fn normal_from_triangle<V: Into<Vec3> + Copy>(v0:V, v1:V, v2:V) -> Vec3 {
    (v1.into() - v0.into()).cross(v2.into() - v0.into()).normalize()
}



pub trait Vector4Extension {
    fn homogenize(self) -> Self;
}

impl Vector4Extension for Vec4 {

    fn homogenize(self) -> Self {
        let [x, y, z, w] = self.into();
        Self::new(x/w, y/w, z/w, w)
    }
}


// helper macro
macro_rules! multi_impl {
    ( $trait:ident for $($type:ty),+ => $tokens:tt) => {
        $( impl $trait for $type $tokens )+
    }
}


pub trait FromUniformScaleExtension: Sized {
    fn from_uniform_scale(scale: f32) -> Self;
}


impl FromUniformScaleExtension for Mat4 {
    fn from_uniform_scale(scale: f32) -> Self {
        Self::from_scale(vec3(scale, scale, scale))
    }
}


pub trait SquareMatrixExtension: Sized {
    fn within(&self, outer: &Self) -> Option<Self>;
}


multi_impl! {
    SquareMatrixExtension for Mat4, Mat3, Mat3A, Mat2 => {
        fn within(&self, outer: &Self) -> Option<Self> {
            if outer.determinant() == 0.0 { None }
            else { Some(outer.inverse() * (*self) * (*outer)) }
        }
    }
}