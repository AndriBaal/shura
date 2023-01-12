use crate::Dimension;

pub use nalgebra::Point2 as Point;
pub use nalgebra::Vector2 as Vector;
pub use nalgebra::UnitVector2 as UnitVector;
pub use nalgebra::Isometry2 as Isometry;
pub use nalgebra::UnitComplex as Rotation;
pub use nalgebra::Translation2 as Translation;
pub use nalgebra::Vector3 as SpacialVector;

#[inline]
pub const fn vector<T>(x: T, y: T) -> Vector<T> {
    return Vector::new(x, y);
}

#[inline]
pub const fn dim<T>(x: T, y: T) -> Dimension<T> {
    return Dimension::new(x, y);
}

pub mod na {
    pub use nalgebra::*;
}
