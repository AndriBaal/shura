pub use nalgebra::Isometry2 as Isometry;
pub use nalgebra::Point2 as Point;
pub use nalgebra::Rotation2 as RotationMatrix;
pub use nalgebra::Translation2 as Translation;
pub use nalgebra::UnitComplex as Rotation;
pub use nalgebra::UnitVector2 as UnitVector;
pub use nalgebra::Vector2 as Vector;
pub use nalgebra::Vector3;
pub use nalgebra::Vector4;

pub const fn vector<T>(x: T, y: T) -> Vector<T> {
    return Vector::new(x, y);
}
