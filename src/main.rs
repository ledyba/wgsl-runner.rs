use std::future::Future;
use std::pin::Pin;
use log::info;

pub mod png;
pub mod render;

// https://github.com/gfx-rs/wgpu-rs/tree/master/examples/capture

fn main() -> anyhow::Result<()> {
  env_logger::init_from_env(env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"));
  let rt =
    tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()?;
  let width = 512;
  let height = 512;
  rt.block_on::<Pin<Box<dyn Future<Output=anyhow::Result<()>>>>>(Box::pin(async {
    let instance = wgpu::Instance::new(wgpu::Backends::all());
    info!("{} adapters found.", instance.enumerate_adapters(wgpu::Backends::all()).collect::<Vec<_>>().len());
    let adapter =
      instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await
        .ok_or(anyhow::Error::msg("Adapter not found."))?;
    let (device, queue) = adapter
      .request_device(
        &wgpu::DeviceDescriptor {
          label: None,
          features: wgpu::Features::empty(),
          limits: wgpu::Limits::default(),
        },
        None,
      )
      .await?;
    let buffer_dimensions = render::BufferDimensions::new(width, height);
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: None,
      size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height) as u64,
      usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    let texture_extent = wgpu::Extent3d {
      width: buffer_dimensions.width as u32,
      height: buffer_dimensions.height as u32,
      depth_or_array_layers: 1,
    };
    // The render pipeline renders data into this texture
    let texture = device.create_texture(&wgpu::TextureDescriptor {
      size: texture_extent,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Rgba8UnormSrgb,
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
      label: None,
    });
    // Set the background to be red
    let command_buffer = {
      let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
      encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[wgpu::RenderPassColorAttachment {
          view: &texture.create_view(&wgpu::TextureViewDescriptor::default()),
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::RED),
            store: true,
          },
        }],
        depth_stencil_attachment: None,
      });

      // Copy the data from the texture to the buffer
      encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
          texture: &texture,
          mip_level: 0,
          origin: wgpu::Origin3d::ZERO,
          aspect: Default::default()
        },
        wgpu::ImageCopyBuffer {
          buffer: &output_buffer,
          layout: wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(
              std::num::NonZeroU32::new(buffer_dimensions.padded_bytes_per_row as u32)
                .unwrap(),
            ),
            rows_per_image: None,
          },
        },
        texture_extent,
      );
      encoder.finish()
    };
    queue.submit(Some(command_buffer));
    png::save("out.png", device, output_buffer, &buffer_dimensions).await?;
    Ok(())
  }))?;
  Ok(())
}
