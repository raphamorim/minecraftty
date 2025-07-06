use anyhow::Result;
use glam::{Vec2, Vec3};
use noise::{NoiseFn, Perlin};
use crate::geometry::{Geometry, Vertex};

pub const CHUNK_SIZE: usize = 8;
pub const CHUNK_HEIGHT: usize = 8;

#[derive(Debug, Clone, Copy)]
pub enum BlockType {
    Grass,
    Dirt,
    Stone,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub position: Vec3,
    pub block_type: BlockType,
}

pub fn generate_chunk_geometry(
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    chunk_pos: Vec2,
) -> Result<Geometry> {
    let chunk = generate_chunk(chunk_pos);
    
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut index_offset = 0u16;

    // Texture coordinates for different block types
    let grass_side_tc = [[0.0, 0.0], [0.5, 0.5], [0.0, 0.5], [0.5, 0.0]];
    let grass_top_tc = [[0.5, 0.0], [1.0, 0.5], [0.5, 0.5], [1.0, 0.0]];
    let stone_tc = [[0.0, 0.5], [0.5, 1.0], [0.0, 1.0], [0.5, 0.5]];
    let dirt_tc = [[0.5, 0.5], [1.0, 1.0], [0.5, 1.0], [1.0, 0.5]];

    for layer in &chunk {
        for row in layer {
            for block in row {
                let x = block.position.x;
                let y = block.position.y;
                let z = block.position.z;

                let tex_coords = match block.block_type {
                    BlockType::Grass => [
                        grass_side_tc, grass_side_tc, grass_side_tc, grass_side_tc, dirt_tc, grass_top_tc
                    ],
                    BlockType::Dirt => [dirt_tc; 6],
                    BlockType::Stone => [stone_tc; 6],
                };

                // Generate vertices for each face of the cube
                let cube_vertices = vec![
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

                vertices.extend(cube_vertices);

                // Generate indices for the cube (6 faces, 2 triangles each)
                // Match the reference implementation's winding order
                let face_indices = [
                    // Front face
                    [0, 1, 2, 0, 3, 1],
                    // Back face (reversed winding)
                    [6, 5, 4, 5, 7, 4],
                    // Left face
                    [8, 9, 10, 8, 11, 9],
                    // Right face (reversed winding)
                    [14, 13, 12, 13, 15, 12],
                    // Bottom face
                    [16, 17, 18, 16, 19, 17],
                    // Top face (reversed winding)
                    [22, 21, 20, 21, 23, 20],
                ];

                for (face, face_idx) in face_indices.iter().enumerate() {
                    let base = index_offset + (face * 4) as u16;
                    for &idx in face_idx {
                        indices.push(base + idx);
                    }
                }

                index_offset += 24; // 24 vertices per cube
            }
        }
    }

    Geometry::new(device, &vertices, &indices)
}

fn generate_chunk(chunk_pos: Vec2) -> Vec<Vec<Vec<Block>>> {
    let actual_chunk_pos = Vec3::new(chunk_pos.x * CHUNK_SIZE as f32, 0.0, chunk_pos.y * CHUNK_SIZE as f32);
    let perlin = Perlin::new(42);

    let mut chunk = Vec::with_capacity(CHUNK_SIZE);

    for x in 0..CHUNK_SIZE {
        let mut layer = Vec::with_capacity(CHUNK_SIZE);
        
        for z in 0..CHUNK_SIZE {
            let height_noise = perlin.get([
                (x as f64 + actual_chunk_pos.x as f64) / 8.0,
                (z as f64 + actual_chunk_pos.z as f64) / 8.0,
            ]);
            let height = ((height_noise + 1.0) * 2.0 + 3.0) as usize; // Height between 3-7
            
            let mut column = Vec::with_capacity(height);
            
            for y in 0..height {
                let world_pos = actual_chunk_pos + Vec3::new(x as f32, y as f32, z as f32);
                
                // Simple block type assignment
                let block_type = if y == height - 1 {
                    BlockType::Grass // Top layer is always grass
                } else if y > height - 3 {
                    BlockType::Dirt  // Next 2 layers are dirt
                } else {
                    BlockType::Stone // Bottom layers are stone
                };
                
                column.push(Block {
                    position: world_pos,
                    block_type,
                });
            }
            
            layer.push(column);
        }
        
        chunk.push(layer);
    }

    chunk
}