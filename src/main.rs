// use std::{
//     collections::VecDeque,
//     path::Path,
//     sync::{Arc, Mutex, Weak},
//     thread::{self, JoinHandle},
//     time::Duration,
// };

// use ffmpeg::{
//     codec::{
//         decoder::{Decoder as FFmpegDecoder, Video as VideoDecoder},
//         packet::Packet,
//     },
//     format::{context::Input, stream::Stream},
//     media::Type,
//     util::{
//         format::pixel::Pixel as PixelFormat,
//         frame::Video as VideoFrame,
//     },
// };
// use flutter_engine::{texture_registry::ExternalTexture, RuntimeData};
// use log::{error, warn};

// const VIDEO_PACKET_QUEUE_MAX: usize = 1024;

// const QUEUE_FULL_SLEEP: u64 = 50;
// const NO_PACKET_SLEEP: u64 = 10;

// struct VideoState {
//     input: Input,
//     video: Arc<Mutex<VideoStreamData>>,
// }

// struct VideoStreamData {
//     stream: StreamData,
//     width: Option<u32>,
//     height: Option<u32>,
//     texture: Arc<ExternalTexture>,
// }

// struct StreamData {
//     stream_index: usize,
//     decoder: Decoder,
//     time_base: f64,
//     duration: i64,
//     packet_queue: VecDeque<PacketData>,
// }

// enum PacketData{
//     Packet(Packet),
// }

// enum Decoder {
//     Video(VideoDecoder),
// }

// pub struct FFmpegPlayer {
//     uri: String,
//     texture: Arc<ExternalTexture>,
//     state: Option<VideoState>,
//     threads: Vec<JoinHandle<()>>,
// }

// pub struct InitResult {
//     pub duration: i64,
//     pub size: (u32, u32),
// }

// impl FFmpegPlayer {
//     pub fn new(uri: String, texture: Arc<ExternalTexture>) -> Self {
//         Self {
//             uri,
//             texture,
//             state: None,
//             threads: Vec::new(),
//         }
//     }

//     pub fn init(&mut self, rt: RuntimeData) -> InitResult {
//         // First, open the input file. Luckily, FFmpeg supports opening videos from URIs.
//         let input = ffmpeg::format::input(&Path::new(&self.uri)).unwrap();
//         // Now create the video stream data.
//         let video = Arc::new(Mutex::new(VideoStreamData::new(
//             StreamData::new(
//                 &input.streams().base(Type::Video).unwrap(),
//                 Decoder::new_video,
//             ),
//             Arc::clone(&self.texture),
//         )));
//         let weak_video = Arc::downgrade(&video);
//         // get the duration
//         let duration = video.lock().unwrap().stream.duration;
//         // Create the state.
//         let state = Arc::new(Mutex::new(VideoState {
//             input,
//             video,
//         }));
//         let weak_state = Arc::downgrade(&state);

//         let own_rt = rt;
//         // This RuntimeData will be moved into the new thread, so we clone first.
//         let rt = own_rt.clone();
//         self.threads.push(thread::spawn(|| {
//             run_player_thread(weak_state, enqueue_next_packet, rt)
//         }));
//         let rt = own_rt.clone();
//         let weak_video_2 = Weak::clone(&weak_video);
//         self.threads.push(thread::spawn(|| {
//             run_player_thread(weak_video, play_video, rt)
//         }));

//         // Wait until the first frame has been decoded and we know the video size.
//         let mut size = None;
//         while let Some(video) = weak_video_2.upgrade() {
//             let video = video.lock.unwrap();
//             if video.width.is_some() && video.height.is_some() {
//                 size = Some((video.width.unwrap(), video.height.unwrap()));
//                 break;
//             } else {
//                 thread::sleep(Duration::from_millis(5));
//             }
//         }

//         self.state.replace(state);

//         InitResult {
//             duration,
//             size: size.unwrap(),
//         }
//     }
// }

// impl Drop for FFmpegPlayer {
//     fn drop(&mut self) {
//         // Drop the Arc<VideoState> to signal threads to exit.
//         self.state.take();
//         // Wait for each thread to exit and print errors.
//         while let Some(t) = self.threads.pop() {
//             if let Err(err) = t.join() {
//                 warn!("thread exited with error: {:?}", err);
//             }
//         }
//     }
// }

// impl VideoStreamData {
//     fn new(stream: StreamData, texture: Arc<ExternalTexture>) -> Self {
//         Self {
//             stream,
//             width: None,
//             height: None,
//             texture,
//         }
//     }
// }

// impl StreamData {
//     fn new<D: FnOnce(FFmpegDecoder) -> Decoder>(
//         stream: &Stream,
//         decoder_fn: D,
//     ) -> Self {
//         // Get the time base of the stream
//         let time_base = stream.time_base();
//         let time_base = time_base.numerator() as f64 / time_base.denominator() as f64;
//         // Calculate duration in seconds.
//         let duration = stream.duration() as f64 * time_base;
//         // Convert to milliseconds as that's what Flutter expects.
//         let duration = (duration * 1000_f64) as i64;

//         Self {
//             stream_index: stream.index(),
//             decoder: decoder_fn(stream.codec().decoder()),
//             time_base,
//             duration,
//             packet_queue: VecDeque::new(),
//         }
//     }
// }

// impl Decoder {
//     fn new_video(d: FFmpegDecoder) -> Self {
//         Decoder::Video(d.video().unwrap())
//     }
//     fn as_video(&mut self) -> &mut VideoDecoder {
//         if let Decoder::Video(d) = self {
//             d
//         } else {
//             panic!("wrong type")
//         }
//     }
// }

// enum LoopState {
//     Running,
//     Sleep(u64),
//     Exit,
// }

// fn run_player_thread<F, T>(state: Weak<Mutex<T>>, f: F, rt: RuntimeData)
// where
//     F: Fn(&mut T, &RuntimeData) -> LoopState,
// {
//     // We have to exit the loop when the state has been lost.
//     while let Some(state) = state.upgrade() {
//         // Run this in a block to drop the MutexGuard as soon as possible.
//         let loop_state = {
//             let mut state = state.lock().unwrap();
//             f(&mut *state, &rt);
//         };

//         match loop_state {
//             LoopState::Running => (),
//             LoopState::Sleep(millis) => thread::sleep(Duration::from_millis(millis)),
//             LoopState::Exit => break,
//         }
//     }
// }

// fn enqueue_next_packet(state: &mut VideoState, _: &RuntimeData) -> LoopState {
//     let video = state.video.lock().unwrap();
//     if video.stream.packet_queue.len() >= VIDEO_PACKET_QUEUE_MAX {
//         return LoopState::sleep(QUEUE_FULL_SLEEP);
//     }
//     // Drop the MutexGuard while we decode the next packet.
//     drop(video);

//     let packet = state.input.packets().next();
//     let mut video = state.video.lock().unwrap();
//     if let Some((stream, packet)) = packet {
//         let idx = stream.index();
//         if idx == video.stream.stream_index {
//             video.stream.packet_queue.push_back(PacketData::Packet(packet));
//         }
//     } else {
//         // EOF reached
//         return LoopState::Exit;
//     }

//     LoopState::Running
// }

// fn play_video(video: &mut VideoStreamData, rt: &RuntimeData) -> LoopState {
//     // Get a packet from the packet queue.
//     let packet = if let Some(packet) = video.stream.packet_queue.pop_front() {
//         packet
//     } else {
//         return LoopState::Sleep(NO_PACKET_SLEEP);
//     };
//     // Decode this packet into a frame.
//     let decoder = video.stream.decoder.as_video();
//     let mut frame = VideoFrame::empty();
//     match decoder.decode(&packet, &mut frame) {
//         Err(err) => {
//             error!("failed to decode video frame: {}", err);
//             return LoopState::Exit;
//         }
//         Ok(_) => {
//             if frame.format() == PixelFormat::None {
//                 return LoopState::Running;
//             }
//         }
//     }
//     // Store the frame size.
//     video.width.replace(frame.width());
//     video.height.replace(frame.height());
// }


use std::{
    collections::VecDeque,
    path::Path,
    sync::{Arc, Mutex, Weak},
    thread::{self, JoinHandle},
    time::Duration,
};

use ffmpeg::{
    codec::{
        decoder::{Decoder as FFmpegDecoder, Video as VideoDecoder},
        packet::Packet,
    },
    format::{context::Input, stream::Stream},
    media::Type,
    util::{
        format::pixel::Pixel as PixelFormat,
        frame::Video as VideoFrame,
    },
};
use flutter_engine::{texture_registry::ExternalTexture, RuntimeData};
use log::{error, warn};

const VIDEO_PACKET_QUEUE_MAX: usize = 1024;

const QUEUE_FULL_SLEEP: u64 = 50;
const NO_PACKET_SLEEP: u64 = 10;

struct VideoState {
    input: Input,
    video: Arc<Mutex<VideoStreamData>>,
}

struct VideoStreamData {
    stream: StreamData,
    width: Option<u32>,
    height: Option<u32>,
    texture: Arc<ExternalTexture>,
}

struct StreamData {
    stream_index: usize,
    decoder: Decoder,
    time_base: f64,
    duration: i64,
    packet_queue: VecDeque<PacketData>,
}

enum PacketData {
    Packet(Packet),
}

enum Decoder {
    Video(VideoDecoder),
}

pub struct FFmpegPlayer {
    uri: String,
    texture: Arc<ExternalTexture>,
    state: Option<VideoState>,
    threads: Vec<JoinHandle<()>>,
}

pub struct InitResult {
    pub duration: i64,
    pub size: (u32, u32),
}

impl FFmpegPlayer {
    pub fn new(uri: String, texture: Arc<ExternalTexture>) -> Self {
        Self {
            uri,
            texture,
            state: None,
            threads: Vec::new(),
        }
    }

    pub fn init(&mut self, rt: RuntimeData) -> InitResult {
        // First, open the input file. Luckily, FFmpeg supports opening videos from URIs.
        let input = ffmpeg::format::input(&Path::new(&self.uri)).unwrap();
        // Now create the video stream data.
        let video = Arc::new(Mutex::new(VideoStreamData::new(
            StreamData::new(
                &input.streams().base(Type::Video).unwrap(),
                Decoder::new_video,
            ),
            Arc::clone(&self.texture),
        )));
        let weak_video = Arc::downgrade(&video);
        // get the duration
        let duration = video.lock().unwrap().stream.duration;
        // Create the state.
        let state = Arc::new(Mutex::new(VideoState {
            input,
            video,
        }));
        let weak_state = Arc::downgrade(&state);

        let own_rt = rt;
        // This RuntimeData will be moved into the new thread, so we clone first.
        let rt = own_rt.clone();
        self.threads.push(thread::spawn(|| {
            run_player_thread(weak_state, enqueue_next_packet, rt)
        }));
        let rt = own_rt.clone();
        let weak_video_2 = Weak::clone(&weak_video);
        self.threads.push(thread::spawn(|| {
            run_player_thread(weak_video, play_video, rt)
        }));

        // Wait until the first frame has been decoded and we know the video size.
        let mut size = None;
        while let Some(video) = weak_video_2.upgrade() {
            let video = video.lock().unwrap();
            if video.width.is_some() && video.height.is_some() {
                size = Some((video.width.unwrap(), video.height.unwrap()));
                break;
            } else {
                thread::sleep(Duration::from_millis(5));
            }
        }

        self.state.replace(state);

        InitResult {
            duration,
            size: size.unwrap(),
        }
    }
}

impl Drop for FFmpegPlayer {
    fn drop(&mut self) {
        // Drop the Arc<VideoState> to signal threads to exit.
        self.state.take();
        // Wait for each thread to exit and print errors.
        while let Some(t) = self.threads.pop() {
            if let Err(err) = t.join() {
                warn!("thread exited with error: {:?}", err);
            }
        }
    }
}

impl VideoStreamData {
    fn new(stream: StreamData, texture: Arc<ExternalTexture>) -> Self {
        Self {
            stream,
            width: None,
            height: None,
            texture,
        }
    }
}

impl StreamData {
    fn new<D: FnOnce(FFmpegDecoder) -> Decoder>(
        stream: &Stream,
        decoder_fn: D,
    ) -> Self {
        // Get the time base of the stream
        let time_base = stream.time_base();
        let time_base = time_base.numerator() as f64 / time_base.denominator() as f64;
        // Calculate duration in seconds.
        let duration = stream.duration() as f64 * time_base;
        // Convert to milliseconds as that's what Flutter expects.
        let duration = (duration * 1000_f64) as i64;

        Self {
            stream_index: stream.index(),
            decoder: decoder_fn(stream.codec().decoder()),
            time_base,
            duration,
            packet_queue: VecDeque::new(),
        }
    }
}

impl Decoder {
    fn new_video(d: FFmpegDecoder) -> Self {
        Decoder::Video(d.video().unwrap())
    }
    fn as_video(&mut self) -> &mut VideoDecoder {
        if let Decoder::Video(d) = self {
            d
        } else {
            panic!("wrong type")
        }
    }
}

enum LoopState {
    Running,
    Sleep(u64),
    Exit,
}

fn run_player_thread<F, T>(state: Weak<Mutex<T>>, f: F, rt: RuntimeData)
where
    F: Fn(&mut T, &RuntimeData) -> LoopState,
{
    // We have to exit the loop when the state has been lost.
    while let Some(state) = state.upgrade() {
        // Run this in a block to drop the MutexGuard as soon as possible.
        let loop_state = {
            let mut state = state.lock().unwrap();
            f(&mut *state, &rt)
        };

        match loop_state {
            LoopState::Running => (),
            LoopState::Sleep(millis) => thread::sleep(Duration::from_millis(millis)),
            LoopState::Exit => break,
        }
    }
}

fn enqueue_next_packet(state: &mut VideoState, _: &RuntimeData) -> LoopState {
    let video = state.video.lock().unwrap();
    if video.stream.packet_queue.len() >= VIDEO_PACKET_QUEUE_MAX {
        return LoopState::Sleep(QUEUE_FULL_SLEEP);
    }
    // Drop the MutexGuard while we decode the next packet.
    drop(video);

    let packet = state.input.packets().next();
    let mut video = state.video.lock().unwrap();
    if let Some((stream, packet)) = packet {
        let idx = stream.index();
        if idx == video.stream.stream_index {
            video.stream.packet_queue.push_back(PacketData::Packet(packet));
        }
    } else {
        // EOF reached
        return LoopState::Exit;
    }

    LoopState::Running
}

fn play_video(video: &mut VideoStreamData, rt: &RuntimeData) -> LoopState {
    // Get a packet from the packet queue.
    let packet = if let Some(packet) = video.stream.packet_queue.pop_front() {
        packet
    } else {
        return LoopState::Sleep(NO_PACKET_SLEEP);
    };
    // Decode this packet into a frame.
    let decoder = video.stream.decoder.as_video();
    let mut frame = VideoFrame::empty();
    match decoder.decode(&packet, &mut frame) {
        Err(err) => {
            error!("failed to decode video frame: {}", err);
            return LoopState::Exit;
        }
        Ok(_) => {
            if frame.format() == PixelFormat::None {
                return LoopState::Running;
            }
        }
    }
    // Store the frame size.
    video.width.replace(frame.width());
    video.height.replace(frame.height());

    LoopState::Running
}

fn main() {
    // Example usage:
    let texture = Arc::new(ExternalTexture); // Replace with actual initialization
    let mut player = FFmpegPlayer::new("path_to_video".to_string(), Arc::clone(&texture));
    let rt = RuntimeData; // Replace with actual initialization
    let init_result = player.init(rt);
    println!("Initialized player with duration: {} ms, size: {:?}", init_result.duration, init_result.size);

    // Program continues running until `player` goes out of scope and `main` function returns.
}
