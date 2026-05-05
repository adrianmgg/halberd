// FIXME this whole mess can problably be simplified

// FIXME should this be a [Box<dyn Error>]? an [io::Error]? a bespoke error type for our use case?
pub(crate) type Error = std::io::Error;

pub(crate) type Result<T> = std::result::Result<T, Error>;

pub(crate) trait SpvWriter {
    fn write_word(&mut self, word: u32) -> Result<()>;
}

// pub(crate) trait SpvWriterExt {
//     fn write<W: SpvWritable>(&mut self, writable: &W) -> Result<()>;
//     // {
//     //     writable.write_spv_to(self)
//     // }
// }
//
// // impl SpvWriterExt for

pub(crate) trait ToWord {
    fn to_word(&self) -> u32;
}

impl<T> SpvWriter for T
where T: std::io::Write
{
    fn write_word(&mut self, word: u32) -> Result<()> { self.write_all(&word.to_le_bytes()) }
}

pub(crate) trait SpvWritable {
    fn write_spv_to(&self, writer: &mut dyn SpvWriter) -> Result<()>;
}

impl SpvWritable for u32 {
    fn write_spv_to(&self, writer: &mut dyn SpvWriter) -> Result<()> { writer.write_word(*self) }
}

impl<T> SpvWritable for T
where T: ToWord
{
    fn write_spv_to(&self, writer: &mut dyn SpvWriter) -> Result<()> {
        writer.write_word(self.to_word())
    }
}

impl<T> SpvWritable for Option<T>
where T: SpvWritable
{
    fn write_spv_to(&self, writer: &mut dyn SpvWriter) -> Result<()> {
        match self {
            Some(v) => self.write_spv_to(writer),
            None => Ok(()),
        }
    }
}

impl<T> SpvWritable for Vec<T>
where T: SpvWritable
{
    fn write_spv_to(&self, writer: &mut dyn SpvWriter) -> Result<()> {
        for item in self {
            item.write_spv_to(writer)?;
        }
        Ok(())
    }
}
