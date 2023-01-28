#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{
    ComponentHandle, ComponentManager, Context, CursorManager, Dimension, Gpu, Input, Isometry,
    Matrix, Model, ModelBuilder, Rotation, Scene, Shura, Uniform, Vector, Vertex,
};

const MINIMAL_FOV: f32 = 0.0000001;

/// Every scene has its own camera that can be adjusted. There is also the relative camera that can be
/// selected in the [ComponentConfig](crate::ComponentConfig) with [CameraUse](crate::CameraUse). The
/// relative camera has always the same fov and position where the bottom_left is (-1.0, -1.0) and the top right is (1.0, 1.0).

#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Camera {
    position: Isometry<f32>,
    target: Option<ComponentHandle>,
    vertical_fov: f32,
    ratio: f32,

    #[cfg_attr(feature = "serialize", serde(skip))]
    proj: Matrix,
    #[cfg_attr(feature = "serialize", serde(skip))]
    model: Model,
    #[cfg_attr(feature = "serialize", serde(skip))]
    uniform: Uniform<Matrix>,
}

impl Camera {
    pub fn new(gpu: &Gpu, position: Isometry<f32>, ratio: f32, vertical_fov: f32) -> Self {
        let fov = Dimension::new(vertical_fov * ratio, vertical_fov);
        let proj = Matrix::projection(fov);
        let view = Matrix::view(position);
        Camera {
            ratio,
            target: None,
            position,
            vertical_fov: vertical_fov,
            proj,

            model: Model::new(gpu, ModelBuilder::cuboid(fov / 2.0)),
            uniform: Uniform::new_vertex(gpu, view * proj),
        }
    }

    pub fn apply_target(
        &mut self,
        man: &ComponentManager,
        #[cfg(feature = "physics")] world: &World,
    ) {
        if let Some(target) = self.target() {
            if let Some(component) = man.component_dynamic(&target) {
                let matrix = component.inner().matrix(
                    #[cfg(feature = "physics")]
                    &world,
                );
                self.set_translation(Vector::new(matrix[12], matrix[13]));
            } else {
                self.set_target(None);
            }
        }
    }

    pub fn buffer(&mut self, gpu: &Gpu) {
        let fov = self.fov() / 2.0;
        let vertices = [
            Vertex::new(Vector::new(-fov.width, fov.height), Vector::new(0.0, 0.0)),
            Vertex::new(Vector::new(-fov.width, -fov.height), Vector::new(0.0, 1.0)),
            Vertex::new(Vector::new(fov.width, -fov.height), Vector::new(1.0, 1.0)),
            Vertex::new(Vector::new(fov.width, fov.height), Vector::new(1.0, 0.0)),
        ];
        self.model.write_vertices(gpu, &vertices);

        let view = Matrix::view(self.position);
        let view_projection = view * self.proj;
        self.uniform.write(gpu, view_projection);
    }

    pub(crate) fn resize(&mut self, ratio: f32) {
        self.ratio = ratio;
        self.reset_camera_projection();
    }

    pub(crate) fn reset_camera_projection(&mut self) {
        self.proj = Matrix::projection(self.fov());
    }

    // Getters

    #[inline]
    /// Returns the bottom left and top right corner of the camera. Computes AABB when the camera
    /// is rotated.
    pub fn rect(&self) -> (Vector<f32>, Vector<f32>) {
        fn rotate_point_around_origin(
            origin: Vector<f32>,
            point: Vector<f32>,
            rot: Rotation<f32>,
        ) -> Vector<f32> {
            let sin = rot.sin_angle();
            let cos = rot.cos_angle();
            return Vector::new(
                origin.x + (point.x - origin.x) * cos - (point.y - origin.y) * sin,
                origin.y + (point.x - origin.x) * sin + (point.y - origin.y) * cos,
            );
        }

        let translation = *self.translation();
        let rotation = *self.rotation();
        let fov: Vector<f32> = (self.fov() / 2.0).into();

        let top_right = rotate_point_around_origin(translation, translation + fov, rotation);
        let bottom_left = rotate_point_around_origin(translation, translation - fov, rotation);
        let top_left = rotate_point_around_origin(
            translation,
            translation + Vector::new(-fov.x, fov.y),
            rotation,
        );
        let bottom_right = rotate_point_around_origin(
            translation,
            translation + Vector::new(fov.x, -fov.y),
            rotation,
        );

        let mut xs = [top_right.x, bottom_left.x, top_left.x, bottom_right.x];
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut ys = [top_right.y, bottom_left.y, top_left.y, bottom_right.y];
        ys.sort_by(|a, b| a.partial_cmp(b).unwrap());

        return (
            Vector::new(*xs.first().unwrap(), *ys.first().unwrap()),
            Vector::new(*xs.last().unwrap(), *ys.last().unwrap()),
        );
    }

    #[inline]
    pub const fn position(&self) -> &Isometry<f32> {
        &self.position
    }

    #[inline]
    pub const fn translation(&self) -> &Vector<f32> {
        &self.position.translation.vector
    }

    #[inline]
    pub fn rotation(&self) -> &Rotation<f32> {
        &self.position.rotation
    }

    #[inline]
    pub fn target(&self) -> Option<ComponentHandle> {
        self.target
    }

    #[inline]
    pub fn fov(&self) -> Dimension<f32> {
        Dimension::new(self.vertical_fov * self.ratio, self.vertical_fov)
    }

    pub fn uniform(&self) -> &Uniform<Matrix> {
        &self.uniform
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    // Setters
    #[inline]
    pub fn set_rotation(&mut self, rotation: Rotation<f32>) {
        self.position.rotation = rotation;
    }

    #[inline]
    pub fn set_position(&mut self, position: Isometry<f32>) {
        self.position = position;
    }

    #[inline]
    pub fn set_translation(&mut self, translation: Vector<f32>) {
        self.position.translation.vector = translation;
    }

    #[inline]
    pub fn set_target(&mut self, target: Option<ComponentHandle>) {
        self.target = target;
    }

    #[inline]
    pub fn set_vertical_fov(
        &mut self,
        cursors: &mut CursorManager,
        input: &Input,
        window_size: Dimension<u32>,
        mut new_fov: f32,
    ) {
        if new_fov < MINIMAL_FOV {
            new_fov = MINIMAL_FOV;
        }
        self.vertical_fov = new_fov;
        self.reset_camera_projection();
        cursors.compute(&self.fov(), &window_size, input)
    }

    #[inline]
    pub fn set_horizontal_fov(
        &mut self,
        cursors: &mut CursorManager,
        input: &Input,
        window_size: Dimension<u32>,
        mut new_fov: f32,
    ) {
        if new_fov < MINIMAL_FOV {
            new_fov = MINIMAL_FOV;
        }
        self.vertical_fov = new_fov / self.ratio;
        self.reset_camera_projection();
        cursors.compute(&self.fov(), &window_size, input)
    }
}


#[cfg(feature = "serialize")]
impl<'de> serde::de::DeserializeSeed<'de> for Shura {
    type Value = Camera;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        impl<'de> serde::de::Visitor<'de> for Shura {
            type Value = Camera;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("Test AB")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut position = None;
                let mut target = None;
                let mut vertical_fov = None;
                let mut ratio = None;

                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "position" => position = Some(map.next_value()?),
                        "target" => target = Some(map.next_value()?),
                        "vertical_fov" => vertical_fov = Some(map.next_value()?),
                        "ratio" => ratio = Some(map.next_value()?),
                        _ => {}
                    }
                }

                let mut cam = Camera::new(
                    &self.gpu,
                    position.unwrap(),
                    ratio.unwrap(),
                    vertical_fov.unwrap(),
                );
                cam.target = target;
                return Ok(cam);
            }
        }
        return deserializer.deserialize_struct("", &[], self);
    }
}
