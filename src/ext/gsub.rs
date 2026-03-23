//! Extension trait for [`Gsub`].

use read_fonts::FontRef;
use write_fonts::{BuilderError, FontBuilder, tables::gsub::Gsub};

/// Extension trait for [`Gsub`].
pub trait GsubExt {
    // Required methods.
    /// Required. Get the gsub table reference.
    fn gsub_ref(&self) -> &Gsub;

    /// Required. Get the mutable gsub table reference.
    fn gsub_ref_mut(&mut self) -> &mut Gsub;

    // Helper methods.
    // TODO

    // User-friendly methods.
    /// Create a [`FontBuilder`] based on the given `font` and the gsub table.
    fn to_font_builder<'a>(&self, font: FontRef<'a>) -> Result<FontBuilder<'a>, BuilderError> {
        let mut builder = FontBuilder::new();
        builder
            .add_table(self.gsub_ref())?
            .copy_missing_tables(font);
        Ok(builder)
    }
}

impl GsubExt for Gsub {
    fn gsub_ref(&self) -> &Gsub {
        self
    }

    fn gsub_ref_mut(&mut self) -> &mut Gsub {
        self
    }
}
