#[macro_use]
extern crate serde_derive;
extern crate docopt;
#[macro_use]
extern crate glium;
extern crate image;
extern crate inotify;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use docopt::Docopt;
use glium::{glutin, Surface, Display};
use glium::texture::Texture2d;
use inotify::{
    event_mask,
    watch_mask,
    Inotify,
};

const USAGE: &'static str = "
shadey
Shader testing environment.

Usage:
  shadey <image> <shader>
  shadey (-h | --help)

Options:
  -h --help          Show this screen.
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_image: String,
    arg_shader: String
}

#[derive(PartialEq)]
enum ProgramStatus {
    Done,
    Reload
}

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}
implement_vertex!(Vertex, position, tex_coords);

fn main() {
    let args: Args = Docopt::new(USAGE).
        and_then(|d| d.deserialize()).
        unwrap_or_else(|e| e.exit());

    loop {
        match run_shader(&args) {
            Ok(status) => {
                if status == ProgramStatus::Done {
                    return;
                }
            },
            Err(e) => {
                eprintln!("Error: {}", e);
                return;
            }
        }
    }
}

fn init_display(events_loop: &glutin::EventsLoop) -> Result<Display, &'static str> {
    let window = glutin::WindowBuilder::new().with_title("Shader Toy");
    let context = glutin::ContextBuilder::new();

    Display::new(window, context, &events_loop).
        map_err(|_| "Could not initialize the display.")
}

fn texture_from_path(display: &Display, image_path: &String) -> Result<Texture2d, &'static str> {
    let img = image::open(&Path::new(image_path)).map_err(|_| "Could not open file.")?.to_rgba();
    let dims = img.dimensions();
    let gl_image = glium::texture::RawImage2d::from_raw_rgba_reversed(&img.into_raw(), dims);

    glium::texture::Texture2d::new(display, gl_image).
        map_err(|_| "Could not create texture from image.")
}

fn fullscreen() -> Vec<Vertex> {
    vec![
        Vertex { position: [-1.0, -1.0], tex_coords: [0.0, 0.0] },
        Vertex { position: [-1.0,  1.0], tex_coords: [0.0, 1.0] },
        Vertex { position: [ 1.0,  1.0], tex_coords: [1.0, 1.0] },

        Vertex { position: [-1.0, -1.0], tex_coords: [0.0, 0.0] },
        Vertex { position: [ 1.0,  1.0], tex_coords: [1.0, 1.0] },
        Vertex { position: [ 1.0, -1.0], tex_coords: [1.0, 0.0] }
    ]
}

fn read_shader(shader_path: &String) -> Result<String, &'static str> {
    let mut file = File::open(shader_path).map_err(|_| "Could not open shader path.")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|_| "Could not read shader file.")?;

    Ok(contents)
}

fn run_shader(args: &Args) -> Result<ProgramStatus, &'static str> {
    // Set up inotify
    let mut file_updates = Inotify::init().map_err(|_| "Failed to initialize an inotify.")?;
    file_updates.add_watch(&args.arg_image, watch_mask::MODIFY).
        map_err(|_| "Could not add watch to image file.")?;
    file_updates.add_watch(&args.arg_shader, watch_mask::MODIFY).
        map_err(|_| "Could not add watch to shader file.")?;

    // Set up window
    let mut events_loop = glutin::EventsLoop::new();
    let display = init_display(&events_loop)?;
    let texture = texture_from_path(&display, &args.arg_image)?;
    let shape = fullscreen();

    let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    // Compile shaders
    let vertex_shader_src = include_str!("main.vert");
    let fragment_shader_src = read_shader(&args.arg_shader)?;
    let program = glium::Program::from_source(&display, vertex_shader_src, &fragment_shader_src, None).unwrap();

    let mut closed = false;
    while !closed {
        let uniforms = uniform! {tex: &texture};
        let mut target = display.draw();
        target.clear_color(1.0, 1.0, 1.0, 1.0);
        target.draw(&vertex_buffer, &indices, &program, &uniforms, &Default::default()).
            map_err(|_| "Could not draw shader.")?;
        target.finish().unwrap();

        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::Closed => {
                        closed = true;
                    },
                    _ => ()
                },
                _ => (),
            }
        });

        // Check for file changes
        let mut event_buffer = [0; 1024];
        let events = file_updates.read_events(&mut event_buffer).
            map_err(|_| "Could not read inotify events.")?;

        for event in events {
            if event.mask.contains(event_mask::MODIFY) {
                return Ok(ProgramStatus::Reload);
            }
        }
    }

    Ok(ProgramStatus::Done)
}
