#[derive(Clone, Copy)]
struct Instance {
    _size: f32,
    _col: [f32; 3],
}

fn main() {
    env_logger::init();
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    let hidpi_factor = window.hidpi_factor();
    let size = window.inner_size().to_physical(hidpi_factor);
    let surface = wgpu::Surface::create(&window);
    let adapter = wgpu::Adapter::request(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::Default,
        backends: wgpu::BackendBit::PRIMARY,
    })
    .unwrap();
    let (mut device, mut queue) = adapter.request_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
            anisotropic_filtering: false,
        },
        limits: wgpu::Limits::default(),
    });
    let sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: size.width.round() as u32,
        height: size.height.round() as u32,
        present_mode: wgpu::PresentMode::Vsync,
    };
    let font_bytes: &[u8] = include_bytes!("./DeLarge.ttf");
    let mut glyph_brush = wgpu_glyph::GlyphBrushBuilder::using_font_bytes(font_bytes)
        .build(&mut device, wgpu::TextureFormat::Bgra8UnormSrgb);
    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);
    let mut shader_compiler = shaderc::Compiler::new().unwrap();
    let vertex_shader = shader_compiler
        .compile_into_spirv(
            include_str!("./shader.vert"),
            shaderc::ShaderKind::Vertex,
            "shader.vert",
            "main",
            None,
        )
        .unwrap();
    let fragment_shader = shader_compiler
        .compile_into_spirv(
            include_str!("./shader.frag"),
            shaderc::ShaderKind::Fragment,
            "shader.frag",
            "main",
            None,
        )
        .unwrap();
    let vs_module = device.create_shader_module(vertex_shader.as_binary());
    let fs_module = device.create_shader_module(fragment_shader.as_binary());
    let triangle_data = &[
        (0.4, [1., 0., 0.]),
        (0.1, [0., 0.5, 0.]),
        (0.2, [0., 0., 1.]),
        (0.7, [1., 1., 0.]),
        (0.5, [0., 1., 1.]),
        (0.9, [1., 0., 1.]),
    ];
    let instance_buffer = device
        .create_buffer_mapped::<Instance>(triangle_data.len(), wgpu::BufferUsage::VERTEX)
        .fill_from_slice(
            &triangle_data
                .iter()
                .map(|&(_size, _col)| Instance { _size, _col })
                .collect::<Vec<_>>(),
        );
    let uniform_buffer = device
        .create_buffer_mapped::<u32>(1, wgpu::BufferUsage::UNIFORM)
        .fill_from_slice(&[3]);
    let instance_size = ::std::mem::size_of::<Instance>();
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        bindings: &[wgpu::BindGroupLayoutBinding {
            binding: 0,
            visibility: wgpu::ShaderStage::VERTEX,
            ty: wgpu::BindingType::UniformBuffer { dynamic: false },
        }],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        bindings: &[wgpu::Binding {
            binding: 0,
            resource: wgpu::BindingResource::Buffer {
                buffer: &uniform_buffer,
                range: 0..1,
            },
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
    });
    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: &pipeline_layout,
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.,
            depth_bias_clamp: 0.,
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: None,
        index_format: wgpu::IndexFormat::Uint16,
        vertex_buffers: &[wgpu::VertexBufferDescriptor {
            stride: instance_size as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float3,
                    offset: 4,
                    shader_location: 1,
                },
            ],
        }],
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    });
    event_loop.run(move |event, _, control_flow| match event {
        winit::event::Event::WindowEvent { event, .. } => match event {
            winit::event::WindowEvent::CloseRequested => {
                *control_flow = winit::event_loop::ControlFlow::Exit
            }
            _ => (),
        },
        winit::event::Event::EventsCleared => {
            let frame = swap_chain.get_next_texture();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &frame.view,
                        resolve_target: None,
                        load_op: wgpu::LoadOp::Clear,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: wgpu::Color::GREEN,
                    }],
                    depth_stencil_attachment: None,
                });
                render_pass.set_pipeline(&render_pipeline);
                render_pass.set_bind_group(0, &bind_group, &[]);
                render_pass.set_vertex_buffers(0, &[(&instance_buffer, 0)]);
                render_pass.draw(0..3, 0..triangle_data.len() as u32);
            }
            glyph_brush.queue(wgpu_glyph::Section {
                text: "Triangles",
                screen_position: (200., 100.),
                color: [0., 0., 0., 1.],
                scale: wgpu_glyph::Scale { x: 30., y: 40. },
                bounds: (size.width as f32, size.height as f32),
                ..wgpu_glyph::Section::default()
            });
            glyph_brush
                .draw_queued(
                    &mut device,
                    &mut encoder,
                    &frame.view,
                    size.width.round() as u32,
                    size.height.round() as u32,
                )
                .unwrap();
            queue.submit(&[encoder.finish()]);
        }
        _ => (),
    });
}
