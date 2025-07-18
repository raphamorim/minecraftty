use anyhow::Result;
use crossterm::{
    cursor, execute,
    terminal::{self},
};
use glam::{Mat4, Vec2, Vec3};
use std::io::{stdout, Write};
use wgpu::util::DeviceExt;

mod camera;
mod geometry;
mod material;
mod perlin;
mod renderer;
mod world_gen;

use camera::Camera;
use geometry::Geometry;
use material::Material;
use renderer::Renderer;
use world_gen::generate_chunk_geometry;

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
        // Use fixed terminal size (100x60)
        let (terminal_width, terminal_height) = (100, 60);

        // Renderer resolution matches terminal exactly
        let renderer_width = terminal_width;
        let renderer_height = terminal_height;

        let renderer = Renderer::new(renderer_width, renderer_height).await?;

        let camera = Camera::new(
            renderer_width as f32 / renderer_height as f32,
            Vec3::new(0.0, 10.0, 0.0), // Position camera at reference location
        );

        let uniforms = Uniforms::new();
        let uniform_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[uniforms]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let material = Material::new(&renderer.device, &renderer.queue, &uniform_buffer)?;
        let uniform_bind_group = material.create_bind_group(&renderer.device, &uniform_buffer);

        // Generate chunks like the reference implementation
        let mut geometries = Vec::new();
        let chunk_positions = [
            Vec2::new(0.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(-1.0, -1.0),
        ];
        for chunk_pos in chunk_positions {
            let geometry =
                generate_chunk_geometry(&renderer.device, &renderer.queue, chunk_pos)?;
            geometries.push(geometry);
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
                    Ok(Event::Key(KeyEvent { code, .. })) => match code {
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
                    },
                    Ok(_) => {}  // Other events
                    Err(_) => {} // Ignore input errors
                }
            }
            Ok(false) => {} // No input available
            Err(_) => {}    // Ignore polling errors
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
        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
                render_pass
                    .set_index_buffer(geometry.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..geometry.index_count, 0, 0..1);
            }
        }

        self.renderer
            .queue
            .submit(std::iter::once(encoder.finish()));

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

        // Sample pixels for terminal rendering
        // Each terminal character covers 1 pixel horizontally, 2 pixels vertically
        for terminal_row in 0..self.terminal_height {
            // Move cursor to the beginning of this terminal row
            write!(stdout, "\x1b[{};1H", terminal_row + 1)?;

            for terminal_col in 0..self.terminal_width {
                // Sample 2 vertical pixels for each terminal character
                let renderer_x = terminal_col;
                let renderer_y = terminal_row * 2;

                // Get top pixel
                let top_idx = ((renderer_y * self.renderer.width + renderer_x) * 4) as usize;
                let c1 = if top_idx + 2 < pixels.len() {
                    [
                        pixels[top_idx],
                        pixels[top_idx + 1],
                        pixels[top_idx + 2],
                    ]
                } else {
                    [0, 0, 0]
                };

                // Get bottom pixel
                let bottom_idx = (((renderer_y + 1) * self.renderer.width + renderer_x) * 4) as usize;
                let c2 = if bottom_idx + 2 < pixels.len() {
                    [
                        pixels[bottom_idx],
                        pixels[bottom_idx + 1],
                        pixels[bottom_idx + 2],
                    ]
                } else {
                    c1 // Use top color if bottom doesn't exist
                };

                if prev_color1.is_none()
                    || prev_color1.unwrap() != c1
                    || prev_color2.is_none()
                    || prev_color2.unwrap() != c2
                {
                    // c1 = foreground, c2 = background
                    write!(stdout, "\x1b[38;2;{};{};{}m", c1[0], c1[1], c1[2])?;
                    write!(stdout, "\x1b[48;2;{};{};{}m", c2[0], c2[1], c2[2])?;

                    prev_color1 = Some(c1);
                    prev_color2 = Some(c2);
                }

                write!(stdout, "▀")?;
            }
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

            std::thread::sleep(std::time::Duration::from_millis(33)); // ~30 FPS
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
