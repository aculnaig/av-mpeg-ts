extern crate av_format as format;

use format::{buffer::{AccReader, HlsReader}, demuxer::{Context, Event}};

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
    
    println!("Init read_events()...");

    let mut packet_count = 0;
    let max_packets = 50; // For test only

    loop {
        match context.read_event() {
            Ok(event) => match event {
                Event::NewPacket(packet) => {
                    packet_count += 1;
                    let stream_idx = packet.stream_index;
                    let stream = &context.info.streams[stream_idx as usize];

                    println!(
                        "Packet #{} found. stream_index: {}, PID: 0x{:04X}, Payload size: {} byte(s)",
                        packet_count, stream_idx, stream.id, packet.data.len()
                    );

                    if packet_count >= max_packets {
                        println!("We reach the maximum packets read for the test");
                        break;
                    }
                },

                Event::NewStream(stream) => {
                    println!("New stream found: ID {:?}", stream.id);
                },

                Event::MoreDataNeeded(needed) => {
                    println!("More data needed ({} byte(s) missing", needed);
                },

                Event::Continue => {
                    continue;
                },

                Event::Eof => {
                    println!("EOF reached.");
                    break;
                },
                _ => unreachable!(),
            },
            Err(e) => {
                eprintln!("Error reading event: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}
