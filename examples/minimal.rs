mod common;

use std::sync::Arc;

use goldy::{
    shader::builtins, Buffer, BufferKind, Color, Instance, Lease, LeaseRenderTarget, NodeAccess, RenderPipeline,
    RenderPipelineDesc, RequestAdapterOptions, RuntimeDescriptor, Scheme, ShaderModule, SurfaceConfig, SurfaceExchange,
    TargetLoad, Transaction, Vertex2D,
};

use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    platform::run_on_demand::EventLoopExtRunOnDemand,
    window::Window,
};

struct Gfx {
    ctx: goldy::Context,
    surface: SurfaceExchange,
    pipeline: RenderPipeline,
    vertices: Buffer,
    _shader: ShaderModule,
    scheme: Scheme,
    present: Transaction,
    _rt: Lease<LeaseRenderTarget>,
}

fn create_gfx(window: &Arc<Window>) -> anyhow::Result<Gfx> {
    let runtime = Arc::new(
        Instance::new()?
            .request_adapter(&RequestAdapterOptions::default())?
            .request_runtime(&RuntimeDescriptor::default())?,
    );
    let ctx = runtime.create_context()?;
    let surface = SurfaceExchange::new(&ctx, window, SurfaceConfig::default())?;

    let vertices = runtime.acquire_buffer_with_data(
        &[
            Vertex2D::new(0.0, -0.5, Color::RED),
            Vertex2D::new(-0.5, 0.5, Color::GREEN),
            Vertex2D::new(0.5, 0.5, Color::BLUE),
        ],
        BufferKind::Scattered,
    )?;

    let shader = ShaderModule::from_slang(&runtime, builtins::VERTEX_COLOR_2D)?;
    let pipeline = common::render_pipeline(
        &runtime,
        &shader,
        surface.format(),
        RenderPipelineDesc {
            vertex_layout: Vertex2D::layout(),
            ..Default::default()
        },
    )?;

    let (w, h) = surface.size();
    let rt = ctx.lease_render_target(w.max(1), h.max(1), surface.format(), None)?;

    let mut scheme = Scheme::new(&ctx);
    let mut pass = scheme.render_pass("triangle", &rt, TargetLoad::Clear(Color::BLACK));
    pass.with_parcel(&vertices, NodeAccess::Read);
    pass.set_pipeline(&pipeline);
    pass.set_vertex_buffer(0, &vertices);
    pass.draw(0..3, 0..1);
    pass.finish();

    let present = surface.bind_render_target(&mut scheme, &rt)?;

    Ok(Gfx {
        ctx,
        surface,
        pipeline,
        vertices,
        _shader: shader,
        scheme,
        present,
        _rt: rt,
    })
}

fn main() -> anyhow::Result<()> {
    let mut event_loop = EventLoop::new()?;

    // State that used to live in `App`
    let mut window = None;
    let mut window_id = None;
    let mut gfx = None;

    #[allow(deprecated)]
    event_loop.run_on_demand(|event, event_loop| {
        match event {
            // Create the window and initialise graphics the first time the app is resumed.
            Event::Resumed => {
                if window.is_some() {
                    return;
                }

                let win = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());

                match create_gfx(&win) {
                    Ok(g) => {
                        window_id = Some(win.id());
                        window = Some(win);
                        gfx = Some(g);
                        window.as_ref().unwrap().request_redraw();
                    }
                    Err(e) => {
                        eprintln!("failed to initialise graphics: {e:?}");
                        event_loop.exit();
                    }
                }
            }

            Event::WindowEvent { event, window_id: id } => {
                // Ignore events from any other window.
                if Some(id) != window_id {
                    return;
                }

                let (Some(win), Some(g)) = (&window, &mut gfx) else {
                    return;
                };

                match event {
                    WindowEvent::CloseRequested => event_loop.exit(),

                    WindowEvent::Resized(s) if s.width > 0 && s.height > 0 => {
                        let _ = g.surface.resize(s.width, s.height);
                        let (w, h) = g.surface.size();

                        if let Ok(rt) = g.ctx.lease_render_target(w.max(1), h.max(1), g.surface.format(), None) {
                            let mut scheme = Scheme::new(&g.ctx);
                            let mut pass = scheme.render_pass("triangle", &rt, TargetLoad::Clear(Color::BLACK));
                            pass.with_parcel(&g.vertices, NodeAccess::Read);
                            pass.set_pipeline(&g.pipeline);
                            pass.set_vertex_buffer(0, &g.vertices);
                            pass.draw(0..3, 0..1);
                            pass.finish();

                            if let Ok(present) = g.surface.bind_render_target(&mut scheme, &rt) {
                                (g.scheme, g.present, g._rt) = (scheme, present, rt);
                            }
                        }

                        win.request_redraw();
                    }

                    WindowEvent::RedrawRequested => {
                        if let Ok(mut submission) = g.scheme.submit() {
                            let _ = (&mut submission >> &g.present).take();
                        }
                        win.request_redraw(); // keep the loop running
                    }

                    _ => {}
                }
            }

            _ => {}
        }
    })?;

    Ok(())
}
