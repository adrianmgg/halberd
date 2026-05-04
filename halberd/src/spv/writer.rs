// FIXME this whole mess can problably be simplified

pub(crate) trait SpvWriter {
    type Error;

    fn write_word(&mut self, word: u32) -> Result<(), Self::Error>;

    fn write<W: SpvWritable>(&mut self, writable: &W) -> Result<(), Self::Error> {
        writable.write_spv_to(self)
    }
}

pub(crate) trait ToWord {
    fn to_word(&self) -> u32;
}

impl<T> SpvWriter for T
where T: std::io::Write
{
    type Error = std::io::Error;

    fn write_word(&mut self, word: u32) -> Result<(), Self::Error> {
        self.write_all(&word.to_le_bytes())
    }
}

pub(crate) trait SpvWritable {
    fn write_spv_to<W: SpvWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error>;
}

impl SpvWritable for u32 {
    fn write_spv_to<W: SpvWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        writer.write_word(*self)
    }
}

impl<T> SpvWritable for T
where T: ToWord
{
    fn write_spv_to<W: SpvWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        writer.write_word(self.to_word())
    }
}
