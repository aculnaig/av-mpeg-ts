extern crate av_format as format;

use format::{buffer::{AccReader, HlsReader}, demuxer::{Context}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hls_url = "https://devstreaming-cdn.apple.com/videos/streaming/examples/bipbop_4x3/gear1/prog_index.m3u8";

    // Initialize the HlsReader
    let hls_reader = HlsReader::new(hls_url)?;

    // Wrap into AccReader
    let acc_reader = AccReader::new(hls_reader);

    let demuxer = av_mpeg_ts::demuxer::MPEGTS_DES.create();

    let mut context = Context::new(demuxer, acc_reader);

    match context.read_headers() {
        Ok(_) => {
            println!("read_headers() success!");

            for (index, stream) in context.info.streams.iter().enumerate() {
                println!("---");
                println!("stream_index: {}", index);
                println!("stream_id (PID): {}", stream.id);
                println!("stream_codec_params: {:?}", stream.params);
            }
        },
        Err(err) => println!("read_headers() error: {:?}", err),
    }
    
    Ok(())
}
