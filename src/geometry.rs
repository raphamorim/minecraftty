use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub tex_coord: [f32; 2],
}

impl Vertex {
    pub fn new(position: Vec3, color: Vec3, tex_coord: [f32; 2]) -> Self {
        Self {
            position: position.to_array(),
            color: color.to_array(),
            tex_coord,
        }
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub struct Geometry {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

impl Geometry {
    pub fn new(
        device: &wgpu::Device,
        vertices: &[Vertex],
        indices: &[u16],
    ) -> Result<Self> {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        })
    }

    pub fn cube(device: &wgpu::Device, position: Vec3, block_type: crate::world_gen::BlockType) -> Result<Self> {
        let x = position.x;
        let y = position.y;
        let z = position.z;

        // Texture coordinates for different block types
        let (grass_side_tc, grass_top_tc, stone_tc, dirt_tc) = (
            [[0.0, 0.0], [0.5, 0.5], [0.0, 0.5], [0.5, 0.0]],
            [[0.5, 0.0], [1.0, 0.5], [0.5, 0.5], [1.0, 0.0]],
            [[0.0, 0.5], [0.5, 1.0], [0.0, 1.0], [0.5, 0.5]],
            [[0.5, 0.5], [1.0, 1.0], [0.5, 1.0], [1.0, 0.5]],
        );

        let tex_coords = match block_type {
            crate::world_gen::BlockType::Grass => [
                grass_side_tc, grass_side_tc, grass_side_tc, grass_side_tc, dirt_tc, grass_top_tc
            ],
            crate::world_gen::BlockType::Dirt => [dirt_tc; 6],
            crate::world_gen::BlockType::Stone => [stone_tc; 6],
        };

        let vertices = vec![
            // Front face
            Vertex::new(Vec3::new(x, y + 1.0, z + 1.0), Vec3::new(1.0, 0.0, 0.0), tex_coords[0][0]),
            Vertex::new(Vec3::new(x + 1.0, y, z + 1.0), Vec3::new(0.0, 1.0, 0.0), tex_coords[0][1]),
            Vertex::new(Vec3::new(x, y, z + 1.0), Vec3::new(0.0, 0.0, 1.0), tex_coords[0][2]),
            Vertex::new(Vec3::new(x + 1.0, y + 1.0, z + 1.0), Vec3::new(0.0, 0.0, 1.0), tex_coords[0][3]),
            
            // Back face
            Vertex::new(Vec3::new(x, y + 1.0, z), Vec3::new(1.0, 0.0, 0.0), tex_coords[1][0]),
            Vertex::new(Vec3::new(x + 1.0, y, z), Vec3::new(0.0, 1.0, 0.0), tex_coords[1][1]),
            Vertex::new(Vec3::new(x, y, z), Vec3::new(0.0, 0.0, 1.0), tex_coords[1][2]),
            Vertex::new(Vec3::new(x + 1.0, y + 1.0, z), Vec3::new(0.0, 0.0, 1.0), tex_coords[1][3]),
            
            // Left face
            Vertex::new(Vec3::new(x, y + 1.0, z), Vec3::new(1.0, 0.0, 0.0), tex_coords[2][0]),
            Vertex::new(Vec3::new(x, y, z + 1.0), Vec3::new(0.0, 1.0, 0.0), tex_coords[2][1]),
            Vertex::new(Vec3::new(x, y, z), Vec3::new(0.0, 0.0, 1.0), tex_coords[2][2]),
            Vertex::new(Vec3::new(x, y + 1.0, z + 1.0), Vec3::new(0.0, 0.0, 1.0), tex_coords[2][3]),
            
            // Right face
            Vertex::new(Vec3::new(x + 1.0, y + 1.0, z), Vec3::new(1.0, 0.0, 0.0), tex_coords[3][0]),
            Vertex::new(Vec3::new(x + 1.0, y, z + 1.0), Vec3::new(0.0, 1.0, 0.0), tex_coords[3][1]),
            Vertex::new(Vec3::new(x + 1.0, y, z), Vec3::new(0.0, 0.0, 1.0), tex_coords[3][2]),
            Vertex::new(Vec3::new(x + 1.0, y + 1.0, z + 1.0), Vec3::new(0.0, 0.0, 1.0), tex_coords[3][3]),
            
            // Bottom face
            Vertex::new(Vec3::new(x, y, z + 1.0), Vec3::new(1.0, 0.0, 0.0), tex_coords[4][0]),
            Vertex::new(Vec3::new(x + 1.0, y, z), Vec3::new(0.0, 1.0, 0.0), tex_coords[4][1]),
            Vertex::new(Vec3::new(x, y, z), Vec3::new(0.0, 0.0, 1.0), tex_coords[4][2]),
            Vertex::new(Vec3::new(x + 1.0, y, z + 1.0), Vec3::new(0.0, 0.0, 1.0), tex_coords[4][3]),
            
            // Top face
            Vertex::new(Vec3::new(x, y + 1.0, z + 1.0), Vec3::new(1.0, 0.0, 0.0), tex_coords[5][0]),
            Vertex::new(Vec3::new(x + 1.0, y + 1.0, z), Vec3::new(0.0, 1.0, 0.0), tex_coords[5][1]),
            Vertex::new(Vec3::new(x, y + 1.0, z), Vec3::new(0.0, 0.0, 1.0), tex_coords[5][2]),
            Vertex::new(Vec3::new(x + 1.0, y + 1.0, z + 1.0), Vec3::new(0.0, 0.0, 1.0), tex_coords[5][3]),
        ];

        let indices: Vec<u16> = (0..6)
            .flat_map(|face| {
                let base = face * 4;
                vec![
                    base, base + 1, base + 2,
                    base, base + 3, base + 1,
                ]
            })
            .collect();

        Self::new(device, &vertices, &indices)
    }
}