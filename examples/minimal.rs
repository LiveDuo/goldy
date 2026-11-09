mod common;

use std::sync::Arc;

use goldy::{
    shader::builtins, Buffer, BufferKind, Color, Instance, Lease, LeaseRenderTarget, NodeAccess,
    RenderPipeline, RenderPipelineDesc, RequestAdapterOptions, RuntimeDescriptor, Scheme, ShaderModule,
    SurfaceConfig, SurfaceExchange, TargetLoad, Vertex2D,
};

use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    platform::run_on_demand::EventLoopExtRunOnDemand,
    window::Window,
};

fn main() {
    let mut event_loop = EventLoop::new().unwrap();

    // Declare per-resource storage so they live across events.
    let mut _window = None;

    #[allow(deprecated)]
    event_loop
        .run_on_demand(|event, event_loop| match event {
            Event::Resumed => {
                let win = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());

                let runtime = Arc::new(
                    Instance::new()
                        .unwrap()
                        .request_adapter(&RequestAdapterOptions::default())
                        .unwrap()
                        .request_runtime(&RuntimeDescriptor::default())
                        .unwrap(),
                );
                let ctx = runtime.create_context().unwrap();
                let surface = SurfaceExchange::new(&ctx, &win, SurfaceConfig::default()).unwrap();

                let data = &[
                    Vertex2D::new(0.0, -0.5, Color::RED),
                    Vertex2D::new(-0.5, 0.5, Color::GREEN),
                    Vertex2D::new(0.5, 0.5, Color::BLUE),
                ];
                let vertices = runtime.acquire_buffer_with_data(data, BufferKind::Scattered).unwrap();

                let shader = ShaderModule::from_slang(&runtime, builtins::VERTEX_COLOR_2D).unwrap();
                let pipeline_desc = RenderPipelineDesc {
                    vertex_layout: Vertex2D::layout(),
                    ..Default::default()
                };
                let pipeline =
                    common::render_pipeline(&runtime, &shader, surface.format(), pipeline_desc).unwrap();

                let (w, h) = surface.size();
                let rt = ctx
                    .lease_render_target(w.max(1), h.max(1), surface.format(), None)
                    .unwrap();

                let mut scheme = Scheme::new(&ctx);
                let mut pass = scheme.render_pass("triangle", &rt, TargetLoad::Clear(Color::BLACK));
                pass.with_parcel(&vertices, NodeAccess::Read);
                pass.set_pipeline(&pipeline);
                pass.set_vertex_buffer(0, &vertices);
                pass.draw(0..3, 0..1);
                pass.finish();

                let present = surface.bind_render_target(&mut scheme, &rt).unwrap();
                let mut submission = scheme.submit().unwrap();
                let _ = (&mut submission >> &present).take();

                // Park everything so it survives past this callback.
                _window = Some(win);
            }

            Event::WindowEvent { event, .. } => {
                if let WindowEvent::CloseRequested = event {
                    event_loop.exit();
                }
            }

            _ => {}
        })
        .unwrap();
}
