use crate::{Vector, Point};
use nalgebra::Scalar;
use std::ops::*;
use winit::dpi::{PhysicalSize, Size};

#[repr(C)]
#[derive(PartialEq, Default, Eq, Copy, Clone, Debug, Hash)]
/// 2D Dimension that hold its width and height
pub struct Dimension<T: Scalar> {
    pub width: T,
    pub height: T,
}

impl<T> Into<Vector<T>> for Dimension<T>
where
    T: Scalar,
{
    fn into(self) -> Vector<T> {
        Vector::new(self.width, self.height)
    }
}

impl Into<Size> for Dimension<u32> {
    fn into(self) -> Size {
        return Size::Physical(PhysicalSize::new(self.width, self.height));
    }
}

impl<T> Into<Point<T>> for Dimension<T>
where
    T: Scalar,
{
    fn into(self) -> Point<T> {
        Point::new(self.width, self.height)
    }
}

impl<T> Into<PhysicalSize<T>> for Dimension<T>
where
    T: Scalar,
{
    fn into(self) -> PhysicalSize<T> {
        PhysicalSize::new(self.width, self.height)
    }
}

impl<T> From<PhysicalSize<T>> for Dimension<T>
where
    T: Scalar,
{
    fn from(from: PhysicalSize<T>) -> Dimension<T> {
        Dimension::new(from.width, from.height)
    }
}

impl<T> From<Point<T>> for Dimension<T>
where
    T: Scalar + Copy,
{
    fn from(from: Point<T>) -> Dimension<T> {
        Dimension::new(from.x, from.y)
    }
}

impl<T> From<Vector<T>> for Dimension<T>
where
    T: Scalar + Copy,
{
    fn from(from: Vector<T>) -> Dimension<T> {
        Dimension::new(from.x, from.y)
    }
}

impl Into<wgpu::Extent3d> for Dimension<u32> {
    fn into(self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
        }
    }
}

impl Into<Dimension<f32>> for Dimension<u32> {
    fn into(self) -> Dimension<f32> {
        Dimension { width: self.width as f32, height: self.height as f32 }
    }
}


impl Into<Dimension<u32>> for Dimension<f32> {
    fn into(self) -> Dimension<u32> {
        Dimension { width: self.width as u32, height: self.height as u32 }
    }
}

macro_rules! impl_dimension {
    ($DimN:ident { $($field:ident),+ }, $n:expr) => {

        impl <T>$DimN<T>
        where T: Scalar {
            #[inline]
            pub const fn new($($field: T),+) -> $DimN<T> {
                $DimN { $($field: $field),+ }
            }

            pub fn set(&mut self, $($field: T),+) {
                $(self.$field = $field);+
            }
        }


        // Arithmetic

        impl <T: Add<Output = T>> Add for $DimN<T>
        where T: Scalar {
            type Output = $DimN<T>;
            fn add(self, v: $DimN<T>) -> $DimN<T> {
                $DimN::new($(self.$field + v.$field),+)
            }
        }

        impl <T: Sub<Output = T>> Sub for $DimN<T>
        where T: Scalar {
            type Output = $DimN<T>;
            fn sub(self, v: $DimN<T>) -> $DimN<T> {
                $DimN::new($(self.$field - v.$field),+)
            }
        }

        impl <T: Div<Output = T>> Div for $DimN<T>
        where T: Scalar {
            type Output = $DimN<T>;
            fn div(self, v: $DimN<T>) -> $DimN<T> {
                $DimN::new($(self.$field / v.$field),+)
            }
        }

        impl <T: Mul<Output = T>> Mul for $DimN<T>
        where T: Scalar {
            type Output = $DimN<T>;
            fn mul(self, v: $DimN<T>) -> $DimN<T> {
                $DimN::new($(self.$field * v.$field),+)
            }
        }

        impl <T: Rem<Output = T>> Rem for $DimN<T>
        where T: Scalar {
            type Output = $DimN<T>;
            fn rem(self, v: $DimN<T>) -> $DimN<T> {
                $DimN::new($(self.$field % v.$field),+)
            }
        }

        // Arithmetic assignment

        impl <T: AddAssign<T>> AddAssign<$DimN<T>> for $DimN<T>
        where T: Scalar {
            fn add_assign(&mut self, v: $DimN<T>) {
                ($(self.$field += v.$field),+);
            }
        }

        impl <T: SubAssign<T>> SubAssign<$DimN<T>> for $DimN<T>
        where T: Scalar {
            fn sub_assign(&mut self, v: $DimN<T>) {
                ($(self.$field -= v.$field),+);
            }
        }

        impl <T: MulAssign<T>> MulAssign<$DimN<T>> for $DimN<T>
        where T: Scalar {
            fn mul_assign(&mut self, v: $DimN<T>) {
                ($(self.$field *= v.$field),+);
            }
        }

        impl <T: DivAssign<T>> DivAssign<$DimN<T>> for $DimN<T>
        where T: Scalar {
            fn div_assign(&mut self, v: $DimN<T>) {
                ($(self.$field /= v.$field),+);
            }
        }

        impl <T: RemAssign<T>> RemAssign<$DimN<T>> for $DimN<T>
        where T: Scalar {
            fn rem_assign(&mut self, v: $DimN<T>) {
                ($(self.$field %= v.$field),+);
            }
        }

        // Other

        impl <S: Neg<Output = S>> Neg for $DimN<S>
        where S: Scalar {
            type Output = $DimN<S>;

            #[inline]
            fn neg(self) -> $DimN<S> {
                $DimN::new($(-self.$field),+)
            }
        }

        unsafe impl<T: bytemuck::Zeroable + Copy + 'static> bytemuck::Pod for $DimN<T> where T: Scalar {}
        unsafe impl<T> bytemuck::Zeroable for $DimN<T> where T: Scalar{}


        impl <T: Copy + Clone + Div<Output = T>>Div<T> for $DimN<T>
        where T: Scalar {
            type Output = $DimN<T>;
            fn div(self, v: T) -> $DimN<T> {
                $DimN::new($(self.$field / v),+)
            }
        }

        impl <T: Copy + Clone + Mul<Output = T>>Mul<T> for $DimN<T>
        where T: Scalar {
            type Output = $DimN<T>;
            fn mul(self, v: T) -> $DimN<T> {
                $DimN::new($(self.$field * v),+)
            }
        }

        impl <T: Copy + Clone + Rem<Output = T>> Rem<T> for $DimN<T>
        where T: Scalar {
            type Output = $DimN<T>;
            fn rem(self, v: T) -> $DimN<T> {
                $DimN::new($(self.$field % v),+)
            }
        }

        impl <T: Copy + Clone + MulAssign<T>> MulAssign<T> for $DimN<T>
        where T: Scalar {
            fn mul_assign(&mut self, v: T) {
                ($(self.$field *= v),+);
            }
        }

        impl <T: Copy + Clone + DivAssign<T>> DivAssign<T> for $DimN<T>
        where T: Scalar {
            fn div_assign(&mut self, v: T) {
                ($(self.$field /= v),+);
            }
        }

        impl <T: Copy + Clone + RemAssign<T>> RemAssign<T> for $DimN<T>
        where T: Scalar {
            fn rem_assign(&mut self, v: T) {
                ($(self.$field %= v),+);
            }
        }
    };
}

impl_dimension!(Dimension { width, height }, 2);
