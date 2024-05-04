use std::io::{BufReader, Cursor};

use crate::{
    graphics::{Gpu, Index, Mesh3D, MeshBuilder3D, Sprite, SpriteBuilder, Vertex3D},
    io::AssetManager,
    math::{Vector2, Vector3},
};

pub struct ModelBuilder {
    pub meshes: Vec<tobj::Model>,
    pub sprites: Vec<Vec<u8>>,
}

impl ModelBuilder {
    pub fn asset(assets: &dyn AssetManager, path: &str) -> Self {
        let obj_text = assets.load_string(path).unwrap();
        let obj_cursor = Cursor::new(&obj_text);
        let mut obj_reader = BufReader::new(obj_cursor);
        let mut path_buf: std::path::PathBuf = path.into();
        path_buf.pop();
        let path_buf = &path_buf;

        let (obj_meshes, obj_materials) = tobj::load_obj_buf(
            &mut obj_reader,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
            |p| {
                let mat_text = assets
                    .load_string(path_buf.join(p).to_str().unwrap())
                    .unwrap();
                tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
            },
        )
        .unwrap();

        let mut sprites = Vec::new();
        for m in obj_materials.unwrap() {
            sprites.push(
                assets
                    .load_bytes(path_buf.join(m.diffuse_texture.unwrap()).to_str().unwrap())
                    .unwrap(),
            );
        }

        Self {
            meshes: obj_meshes,
            sprites,
        }
    }

    pub fn bytes(obj: &str, mtl: &[(&str, &str)], materials: &[(&str, &[u8])]) -> Self {
        let obj_cursor = Cursor::new(obj);
        let mut obj_reader = BufReader::new(obj_cursor);

        let (obj_meshes, obj_materials) = tobj::load_obj_buf(
            &mut obj_reader,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
            |p| {
                let mtl = mtl
                    .iter()
                    .find(|(key, _)| *key == p.to_str().unwrap())
                    .unwrap();
                tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mtl.1)))
            },
        )
        .unwrap();

        let mut sprites = Vec::new();
        for m in obj_materials.unwrap() {
            let material = materials
                .iter()
                .find(|(key, _)| *key == m.diffuse_texture.clone().unwrap())
                .unwrap();
            sprites.push(material.1.to_vec());
        }

        Self {
            meshes: obj_meshes,
            sprites,
        }
    }
}

pub struct Model {
    pub meshes: Vec<(Option<usize>, Mesh3D)>,
    pub sprites: Vec<Sprite>,
}

impl Model {
    pub fn new(gpu: &Gpu, builder: ModelBuilder) -> Self {
        let sprites = builder
            .sprites
            .into_iter()
            .map(|m| gpu.create_sprite(SpriteBuilder::bytes(&m)))
            .collect::<Vec<_>>();
        let meshes = builder
            .meshes
            .into_iter()
            .map(|m| {
                let vertices = (0..m.mesh.positions.len() / 3)
                    .map(|i| Vertex3D {
                        pos: Vector3::new(
                            m.mesh.positions[i * 3],
                            m.mesh.positions[i * 3 + 1],
                            m.mesh.positions[i * 3 + 2],
                        ),
                        tex: Vector2::new(
                            *m.mesh.texcoords.get(i * 2).unwrap_or(&0.0),
                            *m.mesh.texcoords.get(i * 2 + 1).unwrap_or(&0.0),
                        ),
                        normal: Vector3::new(
                            *m.mesh.normals.get(i * 3).unwrap_or(&0.0),
                            *m.mesh.normals.get(i * 3 + 1).unwrap_or(&0.0),
                            *m.mesh.normals.get(i * 3 + 2).unwrap_or(&0.0),
                        ),
                    })
                    .collect::<Vec<_>>();
                (
                    m.mesh.material_id,
                    gpu.create_mesh(&MeshBuilder3D {
                        vertices,
                        indices: Index::from_vec(m.mesh.indices),
                    }),
                )
            })
            .collect::<Vec<_>>();

        Self { meshes, sprites }
    }
}
