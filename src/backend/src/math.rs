use std::mem;
use std::ops::{Index, IndexMut};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    #[inline(always)]
    pub fn as_array(&self) -> &[f32; 4] {
        let ary: &[f32; 4] = unsafe { mem::transmute(self) };
        ary
    }
    #[inline(always)]
    pub fn as_array_mut(&mut self) -> &mut [f32; 4] {
        let ary: &mut [f32; 4] = unsafe { mem::transmute(self) };
        ary
    }
}

impl Index<usize> for Vec4 {
    type Output = f32;
    #[inline(always)]
    fn index<'a>(&'a self, i: usize) -> &'a f32 {
        self.as_array().index(i)
    }
}

impl IndexMut<usize> for Vec4 {
    #[inline(always)]
    fn index_mut<'a>(&'a mut self, i: usize) -> &'a mut f32 {
        self.as_array_mut().index_mut(i)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Mat2x2 {
    pub c0: Vec2,
    pub c1: Vec2,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Mat3x3 {
    pub c0: Vec3,
    pub c1: Vec3,
    pub c2: Vec3,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Mat4x4 {
    pub c0: Vec4,
    pub c1: Vec4,
    pub c2: Vec4,
    pub c3: Vec4,
}
