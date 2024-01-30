
pub unsafe trait ReadBytes {
    fn read_bytes(&self) -> &[u8] where Self: Sized {
        // SAFETY: must be guaranteed by implementor
        unsafe { core::slice::from_raw_parts(
            self as *const Self as *const u8,
            core::mem::size_of::<Self>()
        ) }
    }
}


// impls

unsafe impl<T: ReadBytes> ReadBytes for &T {
    fn read_bytes(&self) -> &[u8] { (*self).read_bytes() }
}


// slice types

unsafe impl<T: ReadBytes> ReadBytes for &[T] {
    fn read_bytes(&self) -> &[u8] {
        // SAFETY: guaranteed by ReadBytes binding
        unsafe { core::slice::from_raw_parts(
            self.as_ptr() as *const u8,
            self.len() * core::mem::size_of::<T>()
        ) }
    }
}

unsafe impl<T: ReadBytes, const N: usize> ReadBytes for [T; N] {
    fn read_bytes(&self) -> &[u8] {
        // SAFETY: guaranteed by ReadBytes binding
        unsafe { core::slice::from_raw_parts(
            self.as_ptr() as *const u8,
            N * core::mem::size_of::<T>()
        ) }
    }
}



// plain types

macro_rules! impl_read_bytes {
    ($($type:ty),*) => { $(unsafe impl ReadBytes for $type {})* }
}

use wgpu::util::{DrawIndirectArgs, DrawIndexedIndirectArgs, DispatchIndirectArgs};

impl_read_bytes!{
    (), crate::Color,
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64,
    DrawIndirectArgs, DrawIndexedIndirectArgs, DispatchIndirectArgs
}


#[cfg(feature = "math")]
mod impl_proj {
    use super::ReadBytes;
    use glam::*;

    impl_read_bytes!{
        Vec2, Vec3, Vec3A, Vec4,
        Mat2, Mat3, Mat3A, Mat4,
        Quat,
        Affine2, Affine3A,
        DVec2, DVec3, DVec4,
        DMat2, DMat3, DMat4,
        DQuat,
        DAffine2, DAffine3,

        I16Vec2, I16Vec3, I16Vec4,
        U16Vec2, U16Vec3, U16Vec4,
        IVec2, IVec3, IVec4,
        UVec2, UVec3, UVec4,
        I64Vec2, I64Vec3, I64Vec4,
        U64Vec2, U64Vec3, U64Vec4,
        BVec2, BVec3, BVec4
    }
}