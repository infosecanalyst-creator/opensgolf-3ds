#![feature(allocator_api)]

use std::rc::Rc;

use citro3d::math::{ClipPlanes, Matrix4, Projection};
use citro3d::render::{ClearFlags, Frame, ScreenTarget, Target};
use citro3d::texenv;
use citro3d::{attrib, buffer, shader};
use ctru::prelude::*;
use ctru::services::gfx::{RawFrameBuffer, Screen};

#[repr(C)]
#[derive(Copy, Clone)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct Vertex {
    pos: Vec3,
    color: Vec3,
}

static VERTICES: &[Vertex] = &[
    Vertex {
        pos: Vec3::new(200.0, 200.0, -0.5),
        color: Vec3::new(1.0, 0.0, 0.0),
    },
    Vertex {
        pos: Vec3::new(100.0, 40.0, -0.5),
        color: Vec3::new(0.0, 1.0, 0.0),
    },
    Vertex {
        pos: Vec3::new(300.0, 40.0, -0.5),
        color: Vec3::new(0.0, 0.0, 1.0),
    },
];

static SHADER_BYTES: &[u8] = include_bytes!("assets/shader.shbin");

const CLEAR_COLOR: u32 = 0x68_B0_D8_FF;

fn main() {
    let mut soc = Soc::new().expect("failed to get SOC");
    drop(soc.redirect_to_3dslink(true, true));

    println!("soc initialized");

    let gfx = Gfx::new().expect("Couldn't obtain GFX controller");
    let mut hid = Hid::new().expect("Couldn't obtain HID controller");
    let apt = Apt::new().expect("Couldn't obtain APT controller");

    let mut instance = citro3d::Instance::new().expect("failed to initialize Citro3D");

    let mut top_screen = gfx.top_screen.borrow_mut();
    let RawFrameBuffer { width, height, .. } = top_screen.raw_framebuffer();
    
    let mut top_target = instance
        .render_target(width, height, top_screen, None)
        .expect("failed to create render target");

    let mut bottom_screen = gfx.bottom_screen.borrow_mut();
    let RawFrameBuffer { width, height, .. } = bottom_screen.raw_framebuffer();

    let mut bottom_target = instance
        .render_target(width, height, bottom_screen, None)
        .expect("failed to create bottom screen render target");

    let shader = Rc::new(shader::Library::from_bytes(SHADER_BYTES).unwrap());
    let vertex_shader = shader.clone().get_shared(0).unwrap();
    let geometry_shader = shader.get_shared(1).unwrap();

    let mut program = shader::Program::new(vertex_shader).unwrap();
    program.set_geometry_shader(geometry_shader, 6).unwrap();

    let projection_uniform_idx = program.get_geometry_uniform("projection").unwrap();

    let vbo_data = buffer::Buffer::new(VERTICES);

    let mut buf_info = buffer::Info::new();
    let attr_info = prepare_vbos(&mut buf_info, vbo_data);

    let stage0 = texenv::TexEnv::new()
        .src(texenv::Mode::BOTH, texenv::Source::PrimaryColor, None, None)
        .func(texenv::Mode::BOTH, texenv::CombineFunc::Replace);

    let projection = Projection::orthographic(
        0.0..240.0,
        0.0..400.0,
        ClipPlanes {
            near: 0.0,
            far: 1.0,
        },
    ).into();

    while apt.main_loop() {
        hid.scan_input();

        if hid.keys_down().contains(KeyPad::START) {
            break;
        }

        instance.render_frame_with(|mut frame| {
            fn cast_lifetime_to_closure<'frame, T>(x: T) -> T
            where
                T: Fn(&mut Frame<'frame>, &'frame mut ScreenTarget<'_>, &Matrix4),
            {
                x
            }

            let render_to = cast_lifetime_to_closure(|frame, target, projection| {
                target.clear(ClearFlags::ALL, CLEAR_COLOR, 0);

                frame
                    .select_render_target(target)
                    .expect("failed to set render target");
                frame.bind_geometry_uniform(projection_uniform_idx, projection);

                frame.set_texenvs(&[stage0]);

                frame.set_attr_info(&attr_info);

                frame
                    .draw_arrays(buffer::Primitive::GeometryPrim, &buf_info, None)
                    .unwrap();
            });

            // We bind the vertex and geometry shaders.
            frame.bind_program(&program);

            render_to(&mut frame, &mut top_target, &projection);
            render_to(&mut frame, &mut bottom_target, &projection);

            frame
        });
    }
}

fn prepare_vbos(buf_info: &mut buffer::Info, vbo_data: buffer::Buffer) -> attrib::Info {
    // Configure attributes for use with the vertex shader
    let mut attr_info = attrib::Info::new();

    attr_info
        .add_loader(attrib::Register::V0, attrib::Format::Float, 3)
        .unwrap();

    attr_info
        .add_loader(attrib::Register::V1, attrib::Format::Float, 3)
        .unwrap();

    buf_info.add(vbo_data, attr_info.permutation()).unwrap();

    attr_info
}
