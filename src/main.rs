use anyhow::Result;
use crossterm::{
    cursor, execute,
    terminal::{self, Clear, ClearType},
};
use glam::{Mat4, Vec2, Vec3};
use std::io::{stdout, Write, BufWriter};
use wgpu::util::DeviceExt;

mod camera;
mod geometry;
mod material;
mod renderer;
mod world_gen;

use camera::Camera;
use geometry::Geometry;
use material::Material;
use renderer::Renderer;
use world_gen::generate_chunk_geometry;

fn get_terminal_size() -> (u32, u32) {
    match terminal::size() {
        Ok((cols, rows)) => (cols as u32, rows as u32),
        Err(_) => (80, 24), // fallback
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

impl Uniforms {
    fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.get_proj_view_matrix().to_cols_array_2d();
    }
}

struct MinecraftTTY {
    renderer: Renderer,
    camera: Camera,
    geometries: Vec<Geometry>,
    material: Material,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    uniforms: Uniforms,
    terminal_width: u32,
    terminal_height: u32,
}

impl MinecraftTTY {
    async fn new() -> Result<Self> {
        let (terminal_width, terminal_height) = get_terminal_size();
        let renderer = Renderer::new(terminal_width, terminal_height).await?;
        
        let camera = Camera::new(
            terminal_width as f32 / terminal_height as f32,
            Vec3::new(4.0, 6.0, 4.0), // Position camera above and away from origin
        );

        let uniforms = Uniforms::new();
        let uniform_buffer = renderer.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let material = Material::new(&renderer.device, &renderer.queue, &uniform_buffer)?;
        let uniform_bind_group = material.create_bind_group(&renderer.device, &uniform_buffer);

        // Generate some chunks closer to origin
        let mut geometries = Vec::new();
        for x in 0..2 {
            for z in 0..2 {
                let chunk_pos = Vec2::new(x as f32, z as f32);
                let geometry = generate_chunk_geometry(&renderer.device, &renderer.queue, chunk_pos)?;
                geometries.push(geometry);
            }
        }

        Ok(Self {
            renderer,
            camera,
            geometries,
            material,
            uniform_buffer,
            uniform_bind_group,
            uniforms,
            terminal_width,
            terminal_height,
        })
    }

    fn handle_input(&mut self) -> Result<bool> {
        use crossterm::event::{self, Event, KeyCode, KeyEvent};

        // Use non-blocking poll with very short timeout
        match event::poll(std::time::Duration::from_millis(0)) {
            Ok(true) => {
                match event::read() {
                    Ok(Event::Key(KeyEvent { code, .. })) => {
                        match code {
                            KeyCode::Char('x') | KeyCode::Esc => return Ok(false),
                            KeyCode::Char('w') | KeyCode::Up => self.camera.move_forward(0.5),
                            KeyCode::Char('s') | KeyCode::Down => self.camera.move_forward(-0.5),
                            KeyCode::Char('a') | KeyCode::Left => self.camera.move_right(-0.5),
                            KeyCode::Char('d') | KeyCode::Right => self.camera.move_right(0.5),
                            KeyCode::Char('q') => self.camera.move_up(-0.5),
                            KeyCode::Char('e') => self.camera.move_up(0.5),
                            KeyCode::Char('h') => self.camera.rotate_y(-10.0),
                            KeyCode::Char('l') => self.camera.rotate_y(10.0),
                            KeyCode::Char('j') => self.camera.rotate_x(10.0),
                            KeyCode::Char('k') => self.camera.rotate_x(-10.0),
                            _ => {}
                        }
                    }
                    Ok(_) => {} // Other events
                    Err(_) => {} // Ignore input errors
                }
            }
            Ok(false) => {} // No input available
            Err(_) => {} // Ignore polling errors
        }
        Ok(true)
    }

    fn render(&mut self) -> Result<()> {
        // Update uniforms
        self.uniforms.update_view_proj(&self.camera);
        self.renderer.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );

        // Render to texture
        let mut encoder = self.renderer.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.renderer.texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.4,
                            g: 0.7,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.renderer.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.material.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            for geometry in &self.geometries {
                render_pass.set_vertex_buffer(0, geometry.vertex_buffer.slice(..));
                render_pass.set_index_buffer(geometry.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..geometry.index_count, 0, 0..1);
            }
        }

        self.renderer.queue.submit(std::iter::once(encoder.finish()));

        // Copy to CPU and display in terminal
        pollster::block_on(self.present_to_terminal())?;

        Ok(())
    }

    async fn present_to_terminal(&self) -> Result<()> {
        let pixels = self.renderer.read_pixels().await?;
        
        // Use a buffered writer for better performance
        let mut stdout = std::io::BufWriter::new(std::io::stdout());
        
        // Use synchronized update to prevent flickering
        write!(stdout, "\x1b[?2026h")?; // Begin synchronized update
        
        // Move cursor to top-left (don't clear screen every frame)
        write!(stdout, "\x1b[H")?;

        // Track previous colors to avoid unnecessary ANSI code output
        let mut prev_color1: Option<[u8; 3]> = None;
        let mut prev_color2: Option<[u8; 3]> = None;

        // Calculate how many terminal rows we need (each terminal row = 2 pixel rows)
        let terminal_rows_needed = (self.renderer.height + 1) / 2;
        
        // Print the pixels using explicit cursor positioning to avoid line wrapping
        let mut y = 0;
        let mut terminal_row = 1;
        
        while y < self.renderer.height && terminal_row <= terminal_rows_needed {
            // Move cursor to the beginning of this terminal row
            write!(stdout, "\x1b[{};1H", terminal_row)?;
            
            let mut x = 0;
            while x < self.renderer.width {
                // Alpha channel is skipped - exactly like reference
                
                let j = ((y * self.renderer.width + x) * 4) as usize;
                let c1 = if j + 2 < pixels.len() {
                    [pixels[j], pixels[j + 1], pixels[j + 2]]
                } else {
                    [0, 0, 0]
                };

                let j2 = (((y + 1) * self.renderer.width + x) * 4) as usize;
                let c2 = if j2 + 2 < pixels.len() {
                    [pixels[j2], pixels[j2 + 1], pixels[j2 + 2]]
                } else {
                    [0, 0, 0]
                };

                if prev_color1.is_none() || prev_color1.unwrap() != c1 ||
                   prev_color2.is_none() || prev_color2.unwrap() != c2 {
                    // Exactly like reference: c1 = foreground, c2 = background
                    write!(stdout, "\x1b[38;2;{};{};{}m", c1[0], c1[1], c1[2])?;
                    write!(stdout, "\x1b[48;2;{};{};{}m", c2[0], c2[1], c2[2])?;

                    prev_color1 = Some(c1);
                    prev_color2 = Some(c2);
                }

                write!(stdout, "▀")?;

                x += 1;
            }

            y += 2;
            terminal_row += 1;
        }

        // End synchronized update
        write!(stdout, "\x1b[?2026l")?; // End synchronized update
        
        stdout.flush()?;
        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;

        let result = loop {
            // Handle input first for better responsiveness
            if !self.handle_input()? {
                break Ok(());
            }
            
            if let Err(e) = self.render() {
                break Err(e);
            }
            
            std::thread::sleep(std::time::Duration::from_millis(33)); // ~30 FPS instead of 60
        };

        execute!(stdout(), cursor::Show, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;

        result
    }
}

fn main() -> Result<()> {
    env_logger::init();
    
    pollster::block_on(async {
        let mut app = MinecraftTTY::new().await?;
        app.run()
    })
}