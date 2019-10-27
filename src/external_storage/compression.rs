use std::io::{Read, Write};

// we can't use level 9, because compressor memory requirements are
//   370MB for level 8 and
//   674MB for level 9,
// but heroku dyno has only 512MB
const XZ_COMPRESSION_LEVEL: u32 = 8;
const LZ4_COMPRESSION_LEVEL: u32 = 1;

pub fn new_decoder<'a>(reader: impl Read + 'a, filename: &str) -> Box<dyn Read + 'a> {
    use xz2::read::XzDecoder;

    if filename.ends_with(".xz") {
        Box::new(XzDecoder::new(reader))
    } else if filename.ends_with(".lz4") {
        Box::new(lz4::Decoder::new(reader).unwrap())
    } else {
        panic!("Unknown archive extension")
    }
}

pub fn new_encoder<'a>(writer: impl Write + 'a, filename: &str) -> Box<dyn Write + 'a> {
    use xz2::write::XzEncoder;

    if filename.ends_with(".xz") {
        Box::new(XzEncoder::new(writer, XZ_COMPRESSION_LEVEL))
    } else if filename.ends_with(".lz4") {
        let writer = lz4::EncoderBuilder::new()
            .level(LZ4_COMPRESSION_LEVEL)
            .build(writer)
            .unwrap();
        let writer = lz4_wrapper::EncoderWrapper::new(writer);
        Box::new(writer)
    } else {
        panic!("Unknown archive extension")
    }
}

// https://github.com/bozaro/lz4-rs/issues/9#issuecomment-176308348
mod lz4_wrapper {
    use std::io::Write;

    use lz4::Encoder;

    pub struct EncoderWrapper<W: Write> {
        inner: Option<Encoder<W>>,
    }

    impl<W: Write> EncoderWrapper<W> {
        pub fn new(encoder: Encoder<W>) -> Self {
            Self { inner: Some(encoder) }
        }
    }

    impl<W: Write> Write for EncoderWrapper<W> {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.inner.as_mut().unwrap().write(buffer)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.inner.as_mut().unwrap().flush()
        }
    }

    impl<W: Write> Drop for EncoderWrapper<W> {
        fn drop(&mut self) {
            if let Some(inner) = self.inner.take() {
                let (_, result) = inner.finish();
                result.unwrap();
            }
        }
    }
}
