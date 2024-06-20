use std::path::Path;
use ffmpeg_next as ffmpeg;

pub struct VideoPlayer {
    video_stream_index: usize,
    decoder: ffmpeg::decoder::Video,
    format_context: ffmpeg::format::context::Input,
}

impl VideoPlayer {
    pub fn new(path: &Path) -> Result<Self, ffmpeg::Error> {
        ffmpeg::init()?;

        let format_context = ffmpeg::format::input(path)?;
        let stream_index = format_context.streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?
            .index();
        
        let video_stream = format_context.stream(stream_index).ok_or(ffmpeg::Error::StreamNotFound)?;
        let codec_context = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())?;
        let decoder = codec_context.decoder().video()?;

        Ok(Self {
            video_stream_index: stream_index,
            decoder,
            format_context,
        })
    }

    pub fn read_frame(&mut self) -> Result<Option<ffmpeg::util::frame::video::Video>, ffmpeg::Error> {
        for (stream, packet) in self.format_context.packets() {
            if stream.index() == self.video_stream_index {
                self.decoder.send_packet(&packet)?;

                let mut frame = ffmpeg::util::frame::video::Video::empty();
                if self.decoder.receive_frame(&mut frame).is_ok() {
                    return Ok(Some(frame));
                }
            }
        }
        Ok(None)
    }
}



// use std::path::Path;

// use ffmpeg_next as ffmpeg;
// use rodio::{OutputStream, Sink};

// pub struct VideoPlayer {
//     video_stream_index: usize,
//     decoder: ffmpeg::decoder::Video,
//     format_context: ffmpeg::format::context::Input,
// }

// impl VideoPlayer {
//     pub fn new(path: &Path) -> Result<Self, ffmpeg::Error> {
//         ffmpeg::init()?;

//         let mut format_context = ffmpeg::format::input(path)?;
//         let stream_index = format_context.streams()
//             .best(ffmpeg::media::Type::Video)
//             .ok_or(ffmpeg::Error::StreamNotFound)?
//             .index();
        
//         let video_stream = format_context.stream(stream_index).ok_or(ffmpeg::Error::StreamNotFound)?;
//         let codec_context = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())?;
//         let decoder = codec_context.decoder().video()?;

//         Ok(Self {
//             video_stream_index: stream_index,
//             decoder,
//             format_context,
//         })
//     }

//     pub fn read_frame(&mut self) -> Result<Option<ffmpeg::util::frame::video::Video>, ffmpeg::Error> {
//         for (stream, packet) in self.format_context.packets() {
//             if stream.index() == self.video_stream_index {
//                 self.decoder.send_packet(&packet)?;

//                 let mut frame = ffmpeg::util::frame::video::Video::empty();
//                 if self.decoder.receive_frame(&mut frame).is_ok() {
//                     return Ok(Some(frame));
//                 }
//             }
//         }
//         Ok(None)
//     }
// }
