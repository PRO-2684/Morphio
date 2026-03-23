//! Extension trait for [`FontRef`].

use read_fonts::{
    FontRef, ReadError, TableProvider,
    tables::cmap::{CmapSubtable, PlatformId},
    types::NameId,
};

/// The preferred order of `cmap` subtables to use when looking for a Unicode mapping. From [fonttools](https://github.com/fonttools/fonttools/blob/29a392f2b67be8ad0229a75e75893c8bd585d792/Lib/fontTools/ttLib/tables/_c_m_a_p.py#L82-L90).
const CMAP_PREFERENCES: &[(PlatformId, u16)] = &[
    (PlatformId::Windows, 10),
    (PlatformId::Unicode, 6),
    (PlatformId::Unicode, 4),
    (PlatformId::Windows, 1),
    (PlatformId::Unicode, 3),
    (PlatformId::Unicode, 2),
    (PlatformId::Unicode, 1),
    (PlatformId::Unicode, 0),
];

/// Name IDs that are friendly to humans, in order of preference.
const FRIENDLY_NAME_IDS: &[NameId] = &[NameId::UNIQUE_ID, NameId::FULL_NAME, NameId::FAMILY_NAME];

/// Extension trait for [`FontRef`].
pub trait FontRefExt<'a> {
    // Required methods.
    /// Required. Gets reference to [`FontRef`].
    fn font_ref(&self) -> &FontRef<'a>;
    /// Required. Gets mutable reference to [`FontRef`].
    fn font_ref_mut(&mut self) -> &mut FontRef<'a>;

    // Provided methods.
    /// Gets the 'best' Unicode cmap subtable available in the font. From [fonttools](https://github.com/fonttools/fonttools/blob/29a392f2b67be8ad0229a75e75893c8bd585d792/Lib/fontTools/ttLib/tables/_c_m_a_p.py#L115-L119).
    fn best_cmap(&self) -> Result<Option<CmapSubtable<'a>>, ReadError> {
        let cmap = self.font_ref().cmap()?;
        let records = cmap.encoding_records();

        for (platform_id, encoding_id) in CMAP_PREFERENCES {
            if let Some(record) = records.iter().find(|record| {
                record.platform_id() == *platform_id && record.encoding_id() == *encoding_id
            }) {
                let subtable = record.subtable(cmap.offset_data())?;
                return Ok(Some(subtable));
            }
        }

        Ok(None)
    }
    /// Gets a friendly name for the font, if available.
    fn friendly_name(&self) -> Result<Option<String>, ReadError> {
        let name = self.font_ref().name()?;
        let Some(record) = name
            .name_record()
            .iter()
            .find(|record| FRIENDLY_NAME_IDS.contains(&record.name_id()))
        else {
            return Ok(None);
        };
        let string = record.string(name.string_data())?;
        let string = string.chars().filter(|c| !c.is_control()).collect();
        Ok(Some(string))
        // todo: Reference https://github.com/fonttools/fonttools/blob/29a392f2b67be8ad0229a75e75893c8bd585d792/Lib/fontTools/ttLib/tables/_n_a_m_e.py#L171C5-L188C20
    }
}

impl<'a> FontRefExt<'a> for FontRef<'a> {
    fn font_ref(&self) -> &FontRef<'a> {
        self
    }

    fn font_ref_mut(&mut self) -> &mut FontRef<'a> {
        self
    }
}
