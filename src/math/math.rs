pub use nalgebra::{
    Isometry2, Isometry3, Matrix2, Matrix3, Matrix4, Point2, Point3, Translation2, Translation3,
    UnitComplex as Rotation2, UnitQuaternion as Rotation3, UnitVector2, UnitVector3, Vector2,
    Vector3, Vector4,
};

/// Create a 2D Vector2
pub const fn vector2<T>(x: T, y: T) -> Vector2<T> {
    return Vector2::new(x, y);
}
