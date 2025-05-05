/// Used to represent types that could contain an image/frame.
pub trait FrameSize {
    /// Width of the image.
    fn width(&self) -> usize;
    /// Height of the image.
    fn height(&self) -> usize;
    /// Channel count for the image.
    fn chans(&self) -> usize;

    /// Returns ([`FrameSize::width`], [`FrameSize::height`], [`FrameSize::chans`]).
    fn frame_size(&self) -> (usize, usize, usize) {
        (self.width(), self.height(), self.chans())
    }

    /// Returns image size in bytes.
    fn num_bytes(&self) -> usize {
        self.width() * self.height() * self.chans()
    }
}
