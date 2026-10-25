use std::collections::VecDeque;

use av_data::{
    params::{AudioInfo, CodecParams, MediaKind, VideoInfo},
    rational::Rational64,
};
use av_format::{
    buffer::Buffered,
    common::GlobalInfo,
    demuxer::{Demuxer, Descr, Descriptor, Event},
    error::Result,
    stream::Stream,
};
use mpeg_ts::{OwnedTsPacket, TsPacket, ts::SectionReassembler};

pub struct MpegTsDes {
    d: Descr,
}

pub struct MpegTsDemuxer {
    pending_packets: VecDeque<OwnedTsPacket>,
}

impl Demuxer for MpegTsDemuxer {
    fn read_headers(
        &mut self,
        buf: &mut dyn Buffered,
        info: &mut GlobalInfo,
    ) -> Result<std::io::SeekFrom> {
        const TS_PACKET_SIZE: usize = 188;
        let mut pat_reassembler = SectionReassembler::default();
        let mut pmt_pid: Option<u16> = None;
        let mut pmt_reassemlber = SectionReassembler::default();
        let mut pmt_found = false;

        let mut packet_buf = [0u8; TS_PACKET_SIZE];

        for _ in 0..50 {
            let bytes_read = buf.read(&mut packet_buf[..])?;
            if bytes_read < TS_PACKET_SIZE {
                break;
            }

            if let Ok(pkt) = TsPacket::parse(&packet_buf) {
                let pid = pkt.header.pid.value();

                if pid == 0x0000 {
                    if let Some(payload) = pkt.payload {
                        pat_reassembler.feed(payload, pkt.header.pusi);

                        while let Some(section) = pat_reassembler.pop_section() {
                            if section.len() >= 8 {
                                pmt_pid = Some(u16::from_be_bytes([
                                    section[section.len() - 4] & 0x1F,
                                    section[section.len() - 3],
                                ]));
                            }
                        }
                    }
                } else if Some(pid) == pmt_pid {
                    if let Some(payload) = pkt.payload {
                        pmt_reassemlber.feed(payload, pkt.header.pusi);

                        while let Some(section) = pmt_reassemlber.pop_section() {
                            let program_info_length =
                                (((section[10] as usize) & 0x0F) << 8) | (section[11] as usize);

                            let mut pos = 12 + program_info_length;

                            let end = section.len() - 4;

                            while pos + 5 <= end {
                                let stream_type = section[pos];
                                let elementary_pid = (((section[pos + 1] as usize) & 0x1F) << 8)
                                    | (section[pos + 2] as usize);
                                let es_info_length = (((section[pos + 3] as usize) & 0x0F) << 8)
                                    | (section[pos + 4] as usize);

                                let mut params = CodecParams {
                                    kind: None,
                                    codec_id: None,
                                    extradata: None,
                                    bit_rate: 0,
                                    convergence_window: 0,
                                    delay: 0,
                                };

                                let mut supported = true;

                                match stream_type {
                                    0x1B => {
                                        log::trace!(
                                            "Found H.264 (AVC) video, pid: 0x{:04X}",
                                            elementary_pid
                                        );

                                        params.codec_id = Some("H264".to_string());
                                        params.kind = Some(MediaKind::Video(VideoInfo {
                                            width: 0,
                                            height: 0,
                                            format: None,
                                        }));
                                    }

                                    0x0F => {
                                        log::trace!(
                                            "Found AAC audio, pid: 0x{:04X}",
                                            elementary_pid
                                        );

                                        params.codec_id = Some("AAC".to_string());
                                        params.kind = Some(MediaKind::Audio(AudioInfo {
                                            rate: 0,
                                            map: None,
                                            format: None,
                                        }));
                                    }
                                    _ => {
                                        supported = false;
                                        unimplemented!();
                                    }
                                }

                                if supported {
                                    let timebase = Rational64::new(1, 90_000);
                                    let mut stream = Stream::from_params(&params, timebase);

                                    stream.id = elementary_pid as isize;
                                    stream.index = info.streams.len();

                                    info.streams.push(stream);
                                }

                                pos += 5 + es_info_length;
                            }

                            pmt_found = true;
                        }
                    }
                }
            }

            if pmt_found {
                break;
            }
        }
        Ok(std::io::SeekFrom::Start(0))
    }

    fn read_event(&mut self, buf: &mut dyn Buffered) -> Result<(std::io::SeekFrom, Event)> {
        todo!()
    }
}

impl Descriptor for MpegTsDes {
    type OutputDemuxer = MpegTsDemuxer;

    fn create(&self) -> Self::OutputDemuxer {
        MpegTsDemuxer {
            pending_packets: VecDeque::with_capacity(1024),
        }
    }

    fn describe(&self) -> &Descr {
        &self.d
    }

    fn probe(&self, data: &[u8]) -> u8 {
        const TS_PACKET_SIZE: usize = 188;
        const SYNC_BYTE: u8 = 0x47;

        if data.len() < TS_PACKET_SIZE * 3 {
            return 0;
        }

        let mut valid_packets = 0;
        let packets_to_check = 5;

        for i in 0..packets_to_check {
            let offset = i * TS_PACKET_SIZE;

            if offset + TS_PACKET_SIZE <= data.len() {
                if data[offset] == SYNC_BYTE {
                    valid_packets += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if valid_packets >= 4 {
            90
        } else if valid_packets >= 3 {
            70
        } else {
            0
        }
    }
}

pub const MPEGTS_DES: &dyn Descriptor<OutputDemuxer = MpegTsDemuxer> = &MpegTsDes {
    d: Descr {
        name: "mpeg-ts",
        demuxer: "mpeg-ts",
        description: "MPEG-TS demuxer",
        extensions: &["ts", "m2ts", "mts"],
        mime: &["video/mp2t", "video/mpeg", "application/mp2t"],
    },
};
