use sdl2::{
    pixels::PixelFormatEnum,
    rect::Rect,
    render::{Canvas, Texture, TextureCreator},
    video::{Window, WindowContext},
    Sdl,
};
use ffmpeg_next::util::frame::video::Video as FfmpegVideo;

pub struct UI<'a> {
    canvas: Canvas<Window>,
    texture: Texture<'a>,
    _texture_creator: TextureCreator<WindowContext>,
}

impl<'a> UI<'a> {
    pub fn new(sdl_context: &'a Sdl, width: u32, height: u32) -> Self {
        let video_subsystem = sdl_context.video().expect("Failed to initialize SDL video subsystem");

        let window = video_subsystem.window("Video Player", width, height)
            .position_centered()
            .opengl()
            .build()
            .expect("Failed to create window");

        let canvas = window.into_canvas()
            .build()
            .expect("Failed to create canvas");

        let texture_creator = canvas.texture_creator();
        let texture = texture_creator.create_texture_streaming(PixelFormatEnum::YV12, width, height)
            .expect("Failed to create texture");

        UI {
            canvas,
            texture,
            _texture_creator: texture_creator,
        }
    }

    pub fn render_frame(&mut self, frame: &FfmpegVideo) {
        let y = frame.data(0);
        let u = frame.data(1);
        let v = frame.data(2);
        let y_stride = frame.stride(0);
        let u_stride = frame.stride(1);
        let v_stride = frame.stride(2);

        self.texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
            let mut offset = 0;

            for (i, chunk) in buffer.chunks_mut(pitch).enumerate() {
                if i < frame.height() as usize {
                    chunk[..y_stride].copy_from_slice(&y[i * y_stride..(i + 1) * y_stride]);
                    offset += y_stride;
                }
            }

            for (i, chunk) in buffer[offset..].chunks_mut(pitch / 2).enumerate() {
                if i < frame.height() as usize / 2 {
                    chunk[..u_stride].copy_from_slice(&u[i * u_stride..(i + 1) * u_stride]);
                    chunk[u_stride..].copy_from_slice(&v[i * v_stride..(i + 1) * v_stride]);
                }
            }
        }).expect("Failed to lock texture for rendering");

        self.canvas.copy(&self.texture, None, Rect::new(0, 0, 1280, 720))
            .expect("Failed to copy texture to canvas");

        self.canvas.present();
    }
}
