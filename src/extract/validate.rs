use std::io;

use ring::digest::{self, SHA256};

pub struct Validate<R> {
    inner: R,
    digest_context: digest::Context,
    sha256: Vec<u8>,
}

impl<R: io::Read> Validate<R> {
    pub(crate) fn new(inner: R, sha256: Vec<u8>) -> Self {
        Validate {
            inner,
            digest_context: digest::Context::new(&SHA256),
            sha256,
        }
    }

    pub fn validate(self) -> anyhow::Result<()> {
        let checksum = self.digest_context.finish();
        anyhow::ensure!(
            checksum.as_ref() == self.sha256.as_slice(),
            "Checksum mismatch",
        );
        Ok(())
    }
}

impl<R: io::Read> io::Read for Validate<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = self.inner.read(buf)?;
        self.digest_context.update(&buf[..len]);
        Ok(len)
    }
}
