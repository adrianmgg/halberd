// FIXME this whole mess can problably be simplified

pub(crate) trait SpvWriter {
    type Error;

    fn write_word<W: Into<Word>>(&mut self, word: W) -> Result<(), Self::Error>;

    fn write<W: SpvWritable>(&mut self, writable: &W) -> Result<(), Self::Error> {
        writable.write_spv_to(self)
    }
}

impl<T> SpvWriter for T
where T: std::io::Write
{
    type Error = std::io::Error;

    fn write_word<W: Into<Word>>(&mut self, word: W) -> Result<(), Self::Error> {
        self.write_all(&word.into().0.to_le_bytes())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub(crate) struct Word(pub(crate) u32);

impl From<u32> for Word {
    fn from(value: u32) -> Self { Word(value) }
}

pub(crate) trait SpvWritable {
    fn write_spv_to<W: SpvWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error>;
}

impl<T> SpvWritable for T
where T: Into<Word> + Copy
{
    fn write_spv_to<W: SpvWriter + ?Sized>(&self, writer: &mut W) -> Result<(), W::Error> {
        writer.write_word((*self).into())
    }
}
