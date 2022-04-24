use std::pin::Pin;
use std::io::Write;
use tokio::io::AsyncWriteExt;

pub async fn save(
  png_output_path: &str,
  device: wgpu::Device,
  output_buffer: wgpu::Buffer,
  buffer_dimensions: &crate::render::BufferDimensions,
) {
  // Note that we're not calling `.await` here.
  let buffer_slice = output_buffer.slice(..);
  let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

  // Poll the device in a blocking manner so that our future resolves.
  // In an actual application, `device.poll(...)` should
  // be called in an event loop or on another thread.
  device.poll(wgpu::Maintain::Wait);
  buffer_future.await.unwrap();

  let padded_buffer = buffer_slice.get_mapped_range();

  let mut png_encoder = png::Encoder::new(
    std::fs::File::create(png_output_path).unwrap(),
    buffer_dimensions.width as u32,
    buffer_dimensions.height as u32,
  );
  png_encoder.set_depth(png::BitDepth::Eight);
  png_encoder.set_color(png::ColorType::Rgba);
  let mut png_writer = png_encoder
    .write_header()
    .unwrap()
    .into_stream_writer_with_size(buffer_dimensions.unpadded_bytes_per_row)
    .unwrap();

  // from the padded_buffer we write just the unpadded bytes into the image
  for chunk in padded_buffer.chunks(buffer_dimensions.padded_bytes_per_row) {
    png_writer
      .write_all(&chunk[..buffer_dimensions.unpadded_bytes_per_row])
      .unwrap();
  }
  png_writer.finish().unwrap();

  // With the current interface, we have to make sure all mapped views are
  // dropped before we unmap the buffer.
  drop(padded_buffer);

  output_buffer.unmap();
}
