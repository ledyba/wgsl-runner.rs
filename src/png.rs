use std::pin::Pin;
use std::io::Write;
use std::path::Path;

pub async fn save(
  output_path: impl AsRef<Path>,
  device: wgpu::Device,
  output_buffer: wgpu::Buffer,
  buffer_dimensions: &crate::render::BufferDimensions,
) -> anyhow::Result<()>
{
  // Note that we're not calling `.await` here.
  let buffer_slice = output_buffer.slice(..);
  let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

  // Poll the device in a blocking manner so that our future resolves.
  // In an actual application, `device.poll(...)` should
  // be called in an event loop or on another thread.
  device.poll(wgpu::Maintain::Wait);
  buffer_future.await?;

  let padded_buffer = buffer_slice.get_mapped_range();

  let mut encoder = png::Encoder::new(
    std::fs::File::create(output_path).unwrap(),
    buffer_dimensions.width as u32,
    buffer_dimensions.height as u32,
  );
  encoder.set_depth(png::BitDepth::Eight);
  encoder.set_color(png::ColorType::Rgba);
  let mut writer = encoder
    .write_header()
    .unwrap()
    .into_stream_writer_with_size(buffer_dimensions.unpadded_bytes_per_row)
    .unwrap();

  // from the padded_buffer we write just the unpadded bytes into the image
  for chunk in padded_buffer.chunks(buffer_dimensions.padded_bytes_per_row) {
    writer.write_all(&chunk[..buffer_dimensions.unpadded_bytes_per_row])?;
  }
  writer.finish()?;

  // With the current interface, we have to make sure all mapped views are
  // dropped before we unmap the buffer.
  drop(padded_buffer);

  output_buffer.unmap();
  Ok(())
}
