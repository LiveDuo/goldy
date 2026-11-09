use std::sync::Arc;

use goldy::{
    shader::builtins, BufferKind, Color, Instance, NodeAccess, RenderPipeline, RenderPipelineDesc,
    RequestAdapterOptions, RuntimeDescriptor, Scheme, ShaderModule, SurfaceConfig, SurfaceExchange, TargetLoad,
    Vertex2D,
};

use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    platform::run_on_demand::EventLoopExtRunOnDemand,
    window::Window,
};

fn main() {
    let mut event_loop = EventLoop::new().unwrap();

    let mut window = None;

    #[allow(deprecated)]
    event_loop
        .run_on_demand(|event, event_loop| match event {
            Event::Resumed => {
                // surface
                let instance = Instance::new().unwrap();
                let adapter = instance.request_adapter(&RequestAdapterOptions::default()).unwrap();
                let runtime = Arc::new(adapter.request_runtime(&RuntimeDescriptor::default()).unwrap());
                let ctx = runtime.create_context().unwrap();
                let win = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
                let surface = SurfaceExchange::new(&ctx, &win, SurfaceConfig::default()).unwrap();

                // vertices
                let data = &[
                    Vertex2D::new(0.0, -0.5, Color::RED),
                    Vertex2D::new(-0.5, 0.5, Color::GREEN),
                    Vertex2D::new(0.5, 0.5, Color::BLUE),
                ];
                let vertices = runtime.acquire_buffer_with_data(data, BufferKind::Scattered).unwrap();

                // pipeline
                let shader = ShaderModule::from_slang(&runtime, builtins::VERTEX_COLOR_2D).unwrap();
                let pipeline_desc = RenderPipelineDesc {
                    vertex_layout: Vertex2D::layout(),
                    ..Default::default()
                };
                let render_pipeline_desc = RenderPipelineDesc {
                    target_format: surface.format(),
                    ..pipeline_desc
                };
                let pipeline = RenderPipeline::new(&runtime, &shader, &shader, &render_pipeline_desc).unwrap();

                let (w, h) = surface.size();
                let rt = ctx.lease_render_target(w, h, surface.format(), None).unwrap();

                // render pass
                let mut scheme = Scheme::new(&ctx);
                let mut pass = scheme.render_pass("triangle", &rt, TargetLoad::Clear(Color::BLACK));
                pass.with_parcel(&vertices, NodeAccess::Read);
                pass.set_pipeline(&pipeline);
                pass.set_vertex_buffer(0, &vertices);
                pass.draw(0..3, 0..1);
                pass.finish();

                // submit
                let present = surface.bind_render_target(&mut scheme, &rt).unwrap();
                let mut submission = scheme.submit().unwrap();
                let _ = (&mut submission >> &present).take();

                // prevent drop
                window = Some(win);
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
