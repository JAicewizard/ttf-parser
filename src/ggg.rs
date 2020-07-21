//! Common types for GDEF, GPOS and GSUB tables.

#![allow(dead_code)]

use core::convert::TryFrom;

use crate::{GlyphId, Tag, NormalizedCoordinate};
use crate::parser::*;


#[derive(Clone, Copy, Default)]
pub(crate) struct GsubGposTable<'a> {
    pub script: Scripts<'a>,
    pub features: Features<'a>,
    pub lookups: Lookups<'a>,
    pub feature_variations: FeatureVariations<'a>,
}

impl<'a> GsubGposTable<'a> {
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        let mut s = Stream::new(data);
        let version: u32 = s.read()?;
        if !(version == 0x00010000 || version == 0x00010001) {
            return None;
        }

        let script_list_offset: Offset16 = s.read()?;
        let feature_list_offset: Offset16 = s.read()?;
        let lookup_list_offset: Offset16 = s.read()?;

        let mut feature_variations_offset: Option<Offset32> = None;
        if version == 0x00010001 {
            feature_variations_offset = s.read()?;
        }

        let mut table = GsubGposTable::default();

        {
            let data = data.get(script_list_offset.to_usize()..)?;
            let mut s = Stream::new(data);
            let count: u16 = s.read()?;
            table.script = Scripts {
                data,
                records: s.read_array16(count)?,
                index: 0,
            };
        }

        {
            let data = data.get(feature_list_offset.to_usize()..)?;
            let mut s = Stream::new(data);
            let count: u16 = s.read()?;
            table.features = Features {
                data,
                records: s.read_array16(count)?,
                index: 0,
            };
        }

        {
            let data = data.get(lookup_list_offset.to_usize()..)?;
            let mut s = Stream::new(data);
            let count: u16 = s.read()?;
            table.lookups = Lookups {
                data,
                records: s.read_array16(count)?,
                index: 0,
            };
        }

        if let Some(offset) = feature_variations_offset {
            let data = data.get(offset.to_usize()..)?;
            let mut s = Stream::new(data);
            s.skip::<u16>(); // majorVersion
            s.skip::<u16>(); // minorVersion
            let count: u32 = s.read()?;
            let records = s.read_array32(count)?;
            table.feature_variations = FeatureVariations {
                data,
                records,
                index: 0,
            };
        }

        Some(table)
    }
}


/// A generic interface over GSUB/GPOS tables.
pub trait GlyphPosSubTable {
    /// Returns an iterator over GSUB/GPOS table scripts.
    fn scripts(&self) -> Scripts;

    /// Returns a `Script` at `index`.
    ///
    /// Just a shorthand for:
    ///
    /// ```ignore
    /// table.scripts().nth(usize::from(index.0))
    /// ```
    fn script_at(&self, index: ScriptIndex) -> Option<Script> {
        self.scripts().nth(usize::from(index.0))
    }

    /// Returns an iterator over GSUB/GPOS table features.
    fn features(&self) -> Features;

    /// Returns a `Feature` at `index`.
    ///
    /// Just a shorthand for:
    ///
    /// ```ignore
    /// table.features().nth(usize::from(index.0))
    /// ```
    fn feature_at(&self, index: FeatureIndex) -> Option<Feature> {
        self.features().nth(usize::from(index.0))
    }

    /// Returns an iterator over GSUB/GPOS table lookups.
    fn lookups(&self) -> Lookups;

    /// Returns a `Lookup` at `index`.
    ///
    /// Just a shorthand for:
    ///
    /// ```ignore
    /// table.lookups().nth(usize::from(index.0))
    /// ```
    fn lookup_at(&self, index: LookupIndex) -> Option<Lookup> {
        self.lookups().nth(usize::from(index.0))
    }

    /// Returns an iterator over GSUB/GPOS table feature variations.
    ///
    /// Iterator will be empty when font doesn't have Feature Variations data.
    fn feature_variations(&self) -> FeatureVariations;

    /// Returns a `feature_variations` at `index`.
    ///
    /// Just a shorthand for:
    ///
    /// ```ignore
    /// table.feature_variations().nth(usize::from(index.0))
    /// ```
    fn feature_variation_at(&self, index: FeatureVariationIndex) -> Option<FeatureVariation> {
        self.feature_variations().nth(usize::num_from(index.0))
    }
}


/// A type-safe wrapper for script index used by GSUB/GPOS tables.
#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct ScriptIndex(pub u16);


/// A type-safe wrapper for language index used by GSUB/GPOS tables.
#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct LanguageIndex(pub u16);


/// A type-safe wrapper for feature index used by GSUB/GPOS tables.
#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct FeatureIndex(pub u16);

impl FromData for FeatureIndex {
    const SIZE: usize = 2;

    #[inline]
    fn parse(data: &[u8]) -> Option<Self> {
        u16::parse(data).map(FeatureIndex)
    }
}


/// A type-safe wrapper for feature index used by GSUB/GPOS tables.
#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct FeatureVariationIndex(pub u32);


/// A type-safe wrapper for lookup index used by GSUB/GPOS tables.
#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct LookupIndex(pub u16);

impl FromData for LookupIndex {
    const SIZE: usize = 2;

    #[inline]
    fn parse(data: &[u8]) -> Option<Self> {
        u16::parse(data).map(LookupIndex)
    }
}


#[derive(Clone, Copy)]
struct Record {
    tag: Tag,
    offset: Offset16,
}

impl FromData for Record {
    const SIZE: usize = 6;

    #[inline]
    fn parse(data: &[u8]) -> Option<Self> {
        let mut s = Stream::new(data);
        Some(Record {
            tag: s.read()?,
            offset: s.read()?,
        })
    }
}


/// An iterator over GSUB/GPOS table scripts.
#[allow(missing_debug_implementations)]
#[derive(Clone, Copy, Default)]
pub struct Scripts<'a> {
    data: &'a [u8], // GSUB/GPOS data from beginning of ScriptList.
    records: LazyArray16<'a, Record>,
    index: u16,
}

impl<'a> Iterator for Scripts<'a> {
    type Item = Script<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.index = self.index.checked_add(1)?;
        self.nth(usize::from(self.index) - 1)
    }

    fn count(self) -> usize {
        usize::from(self.records.len())
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let record = self.records.get(u16::try_from(n).ok()?)?;
        let data = self.data.get(record.offset.to_usize()..)?;
        let mut s = Stream::new(data);
        let default_lang: Option<Offset16> = s.read()?;
        let count: u16 = s.read()?;
        let records = s.read_array16(count)?;
        Some(Script {
            data,
            script: record.tag,
            default_lang_offset: default_lang,
            records,
        })
    }
}


/// A font script.
#[allow(missing_debug_implementations)]
#[derive(Clone, Copy)]
pub struct Script<'a> {
    data: &'a [u8], // GSUB/GPOS data from beginning of ScriptTable.
    script: Tag,
    default_lang_offset: Option<Offset16>,
    records: LazyArray16<'a, Record>,
}

impl<'a> Script<'a> {
    /// Returns scrips's tag.
    #[inline]
    pub fn tag(&self) -> Tag {
        self.script
    }

    /// Parses scrips's default language.
    pub fn default_language(&self) -> Option<Language> {
        let data = self.data.get(self.default_lang_offset?.to_usize()..)?;
        parse_lang_sys_table(data, None)
    }

    /// Returns an iterator over scrips's languages.
    pub fn languages(&self) -> Languages {
        Languages {
            data: self.data,
            records: self.records,
            index: 0,
        }
    }

    /// Returns a `Language` at `index`.
    ///
    /// Just a shorthand for:
    ///
    /// ```ignore
    /// script.languages().nth(usize::from(index.0))
    /// ```
    pub fn language_at(&self, index: LanguageIndex) -> Option<Language> {
        self.languages().nth(usize::from(index.0))
    }

    /// Returns a `Language` by `tag`.
    ///
    /// Uses binary search and not an iterator internally.
    pub fn language_by_tag(&self, tag: Tag) -> Option<(LanguageIndex, Language)> {
        let (idx, _) = self.records.binary_search_by(|r| r.tag.cmp(&tag))?;
        let lang = self.language_at(LanguageIndex(idx))?;
        Some((LanguageIndex(idx), lang))
    }
}


/// An iterator over GSUB/GPOS table script languages.
#[allow(missing_debug_implementations)]
#[derive(Clone, Copy)]
pub struct Languages<'a> {
    data: &'a [u8], // GSUB/GPOS data from beginning of ScriptTable.
    records: LazyArray16<'a, Record>,
    index: u32,
}

impl<'a> Iterator for Languages<'a> {
    type Item = Language<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.index = self.index.checked_add(1)?;
        self.nth(usize::num_from(self.index) - 1)
    }

    fn count(self) -> usize {
        usize::from(self.records.len())
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let record = self.records.get(u16::try_from(n).ok()?)?;
        let data = self.data.get(record.offset.to_usize()..)?;
        parse_lang_sys_table(data, Some(record.tag))
    }
}

fn parse_lang_sys_table(data: &[u8], tag: Option<Tag>) -> Option<Language> {
    let mut s = Stream::new(data);
    s.skip::<u16>(); // lookupOrder (reserved)

    let required_feature_index = match s.read::<u16>()? {
        0xFFFF => None, // no required features
        n => Some(FeatureIndex(n)),
    };

    let count: u16 = s.read()?;
    Some(Language {
        tag: tag.unwrap_or_else(|| Tag::from_bytes(b"DFLT")),
        required_feature_index,
        feature_indices: s.read_array16(count)?,
    })
}

/// A font language.
#[derive(Clone, Copy, Debug)]
pub struct Language<'a> {
    /// Language tag.
    pub tag: Tag,
    /// Required feature index.
    pub required_feature_index: Option<FeatureIndex>,
    /// List of feature indices associated with this language.
    pub feature_indices: LazyArray16<'a, FeatureIndex>,
}


/// An iterator over GSUB/GPOS table features.
#[allow(missing_debug_implementations)]
#[derive(Clone, Copy, Default)]
pub struct Features<'a> {
    data: &'a [u8], // Data from beginning of FeatureList.
    records: LazyArray16<'a, Record>,
    index: u16,
}

impl<'a> Iterator for Features<'a> {
    type Item = Feature<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.index = self.index.checked_add(1)?;
        self.nth(usize::from(self.index) - 1)
    }

    fn count(self) -> usize {
        usize::from(self.records.len())
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let record = self.records.get(u16::try_from(n).ok()?)?;
        let data = self.data.get(record.offset.to_usize()..)?;
        let mut s = Stream::new(data);
        s.skip::<Offset16>(); // featureParams
        let count: u16 = s.read()?;
        Some(Feature {
            tag: record.tag,
            lookup_indices: s.read_array16(count)?,
        })
    }
}


/// A font feature.
#[derive(Clone, Copy, Debug)]
pub struct Feature<'a> {
    /// Feature tag.
    pub tag: Tag,
    /// List of lookup indices associated with this feature.
    pub lookup_indices: LazyArray16<'a, LookupIndex>,
}


/// An iterator over GSUB/GPOS table lookups.
#[allow(missing_debug_implementations)]
#[derive(Clone, Copy, Default)]
pub struct Lookups<'a> {
    data: &'a [u8], // Data from beginning of LookupList.
    records: LazyArray16<'a, Record>,
    index: u16,
}

impl<'a> Iterator for Lookups<'a> {
    type Item = Lookup<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.index = self.index.checked_add(1)?;
        self.nth(usize::from(self.index) - 1)
    }

    fn count(self) -> usize {
        usize::from(self.records.len())
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let record = self.records.get(u16::try_from(n).ok()?)?;
        let data = self.data.get(record.offset.to_usize()..)?;
        let mut s = Stream::new(data);
        let lookup_type: u16 = s.read()?;
        let lookup_flag: u16 = s.read()?;
        let count: u16 = s.read()?;
        let offsets = s.read_offsets16(count, data)?;
        let mark_filtering_set: u16 = s.read()?;
        Some(Lookup {
            lookup_type,
            lookup_flag,
            offsets,
            mark_filtering_set,
        })
    }
}


/// A font lookup.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct Lookup<'a> {
    lookup_type: u16,
    lookup_flag: u16,
    offsets: Offsets16<'a, Offset16>,
    mark_filtering_set: u16, // TODO: optional
}


#[derive(Clone, Copy)]
struct FeatureVariationRecord {
    condition_set_offset: Offset32,
    feature_table_substitution_offset: Offset32,
}

impl FromData for FeatureVariationRecord {
    const SIZE: usize = 8;

    #[inline]
    fn parse(data: &[u8]) -> Option<Self> {
        let mut s = Stream::new(data);
        Some(FeatureVariationRecord {
            condition_set_offset: s.read()?,
            feature_table_substitution_offset: s.read()?,
        })
    }
}


/// An iterator over GSUB/GPOS table features.
#[allow(missing_debug_implementations)]
#[derive(Clone, Copy, Default)]
pub struct FeatureVariations<'a> {
    data: &'a [u8], // Data from beginning of FeatureVariationsList.
    records: LazyArray32<'a, FeatureVariationRecord>,
    index: u32,
}

impl<'a> Iterator for FeatureVariations<'a> {
    type Item = FeatureVariation<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.index = self.index.checked_add(1)?;
        self.nth(usize::num_from(self.index) - 1)
    }

    fn count(self) -> usize {
        usize::num_from(self.records.len())
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let record = self.records.get(u32::try_from(n).ok()?)?;
        Some(FeatureVariation {
            data: self.data,
            condition_set_offset: record.condition_set_offset,
            feature_table_substitution_offset: record.feature_table_substitution_offset,
        })
    }
}


/// A font feature variation.
#[derive(Clone, Copy, Debug)]
pub struct FeatureVariation<'a> {
    data: &'a [u8], // Data from beginning of FeatureVariations.
    condition_set_offset: Offset32,
    feature_table_substitution_offset: Offset32,
}

impl<'a> FeatureVariation<'a> {
    /// Evaluates variation using specified `coordinates`.
    ///
    /// Number of `coordinates` should be the same as number of variation axes in the font.
    pub fn evaluate(&self, coordinates: &[NormalizedCoordinate]) -> bool {
        for condition in try_opt_or!(self.condition_set(), false) {
            if !condition.evaluate(coordinates) {
                return false;
            }
        }

        true
    }

    fn condition_set(&self) -> Option<ConditionSet<'a>> {
        let data = self.data.get(self.condition_set_offset.to_usize()..)?;
        let mut s = Stream::new(data);
        let count: u16 = s.read()?;
        Some(ConditionSet {
            data,
            offsets: s.read_array16(count)?,
            index: 0,
        })
    }

    /// Parses an iterator over feature variation substitutions.
    pub fn substitutions(&self) -> Option<FeatureSubstitutions<'a>> {
        let data = self.data.get(self.feature_table_substitution_offset.to_usize()..)?;
        let mut s = Stream::new(data);
        s.skip::<u16>(); // majorVersion
        s.skip::<u16>(); // minorVersion
        let count: u16 = s.read()?;
        Some(FeatureSubstitutions {
            data,
            records: s.read_array16(count)?,
            index: 0,
        })
    }
}


#[derive(Clone, Copy, Debug)]
struct ConditionSet<'a> {
    data: &'a [u8], // Data from beginning of ConditionSet.
    offsets: LazyArray16<'a, Offset32>,
    index: u16,
}

impl<'a> Iterator for ConditionSet<'a> {
    type Item = Condition;

    fn next(&mut self) -> Option<Self::Item> {
        self.index = self.index.checked_add(1)?;
        self.nth(usize::from(self.index) - 1)
    }

    fn count(self) -> usize {
        usize::from(self.offsets.len())
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let offset = self.offsets.get(u16::try_from(n).ok()?)?;
        let condition: Condition = Stream::read_at(self.data, offset.to_usize())?;
        if condition.format != 1 {
            return None;
        }

        Some(condition)
    }
}


#[derive(Clone, Copy)]
struct Condition {
    format: u16,
    axis_index: u16,
    filter_range_min_value: i16,
    filter_range_max_value: i16,
}

impl Condition {
    fn evaluate(&self, coordinates: &[NormalizedCoordinate]) -> bool {
        let coord = coordinates.get(usize::from(self.axis_index)).cloned().unwrap_or_default();
        self.filter_range_min_value <= coord.get() && coord.get() <= self.filter_range_max_value
    }
}

impl FromData for Condition {
    const SIZE: usize = 8;

    #[inline]
    fn parse(data: &[u8]) -> Option<Self> {
        let mut s = Stream::new(data);
        Some(Condition {
            format: s.read()?,
            axis_index: s.read()?,
            filter_range_min_value: s.read()?,
            filter_range_max_value: s.read()?,
        })
    }
}


/// An iterator over GSUB/GPOS table features.
#[derive(Clone, Copy, Debug)]
pub struct FeatureSubstitutions<'a> {
    data: &'a [u8], // Data from beginning of FeatureVariationsList.
    records: LazyArray16<'a, FeatureTableSubstitutionRecord>,
    index: u16,
}

impl<'a> Iterator for FeatureSubstitutions<'a> {
    type Item = FeatureSubstitution<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.index = self.index.checked_add(1)?;
        self.nth(usize::from(self.index) - 1)
    }

    fn count(self) -> usize {
        usize::from(self.records.len())
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let record = self.records.get(u16::try_from(n).ok()?)?;
        Some(FeatureSubstitution {
            data: self.data,
            index: record.index,
            table_offset: record.table_offset,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct FeatureTableSubstitutionRecord {
    index: FeatureIndex,
    table_offset: Offset32,
}

impl FromData for FeatureTableSubstitutionRecord {
    const SIZE: usize = 6;

    fn parse(data: &[u8]) -> Option<Self> {
        let mut s = Stream::new(data);
        Some(FeatureTableSubstitutionRecord {
            index: s.read()?,
            table_offset: s.read()?,
        })
    }
}


/// A font feature substitution.
#[derive(Clone, Copy, Debug)]
pub struct FeatureSubstitution<'a> {
    data: &'a [u8], // Data from beginning of FeatureTableSubstitution.
    index: FeatureIndex,
    table_offset: Offset32,
}

impl<'a> FeatureSubstitution<'a> {
    /// Returns feature index.
    pub fn index(&self) -> FeatureIndex {
        self.index
    }

    /// Parses substitution's feature.
    pub fn feature(&self) -> Option<Feature<'a>> {
        let data = self.data.get(self.table_offset.to_usize()..)?;
        let mut s = Stream::new(data);
        s.skip::<u16>(); // featureParams (reserved)
        let count: u16 = s.read()?;
        Some(Feature {
            tag: Tag(0),
            lookup_indices: s.read_array16(count)?,
        })
    }
}


#[derive(Clone, Copy)]
struct RangeRecord {
    start_glyph_id: GlyphId,
    end_glyph_id: GlyphId,
    value: u16,
}

impl RangeRecord {
    fn range(&self) -> core::ops::RangeInclusive<GlyphId> {
        self.start_glyph_id..=self.end_glyph_id
    }
}

impl FromData for RangeRecord {
    const SIZE: usize = 6;

    #[inline]
    fn parse(data: &[u8]) -> Option<Self> {
        let mut s = Stream::new(data);
        Some(RangeRecord {
            start_glyph_id: s.read::<GlyphId>()?,
            end_glyph_id: s.read::<GlyphId>()?,
            value: s.read::<u16>()?,
        })
    }
}


/// A [Coverage Table](https://docs.microsoft.com/en-us/typography/opentype/spec/chapter2#coverage-table).
#[derive(Clone, Copy, Debug)]
pub(crate) struct CoverageTable<'a> {
    data: &'a [u8],
}

impl<'a> CoverageTable<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        CoverageTable { data }
    }

    pub fn contains(&self, glyph_id: GlyphId) -> bool {
        let mut s = Stream::new(self.data);
        let format: u16 = try_opt_or!(s.read(), false);

        match format {
            1 => {
                let count = try_opt_or!(s.read::<u16>(), false);
                let records = try_opt_or!(s.read_array16::<GlyphId>(count), false);
                records.binary_search(&glyph_id).is_some()
            }
            2 => {
                let count = try_opt_or!(s.read::<u16>(), false);
                let records = try_opt_or!(s.read_array16::<RangeRecord>(count), false);
                records.into_iter().any(|r| r.range().contains(&glyph_id))
            }
            _ => false,
        }
    }
}


/// A value of [Class Definition Table](https://docs.microsoft.com/en-us/typography/opentype/spec/chapter2#class-definition-table).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Class(pub u16);

impl FromData for Class {
    const SIZE: usize = 2;

    #[inline]
    fn parse(data: &[u8]) -> Option<Self> {
        u16::parse(data).map(Class)
    }
}


/// A [Class Definition Table](https://docs.microsoft.com/en-us/typography/opentype/spec/chapter2#class-definition-table).
#[derive(Clone, Copy)]
pub(crate) struct ClassDefinitionTable<'a> {
    data: &'a [u8],
}

impl<'a> ClassDefinitionTable<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        ClassDefinitionTable { data }
    }

    /// Any glyph not included in the range of covered glyph IDs automatically belongs to Class 0.
    pub fn get(&self, glyph_id: GlyphId) -> Class {
        self.get_impl(glyph_id).unwrap_or(Class(0))
    }

    fn get_impl(&self, glyph_id: GlyphId) -> Option<Class> {
        let mut s = Stream::new(self.data);
        let format: u16 = s.read()?;
        match format {
            1 => {
                let start_glyph_id: GlyphId = s.read()?;

                // Prevent overflow.
                if glyph_id < start_glyph_id {
                    return None;
                }

                let count: u16 = s.read()?;
                let classes = s.read_array16::<Class>(count)?;
                classes.get(glyph_id.0 - start_glyph_id.0)
            }
            2 => {
                let count: u16 = s.read()?;
                let records = s.read_array16::<RangeRecord>(count)?;
                records.into_iter().find(|r| r.range().contains(&glyph_id))
                    .map(|record| Class(record.value))
            }
            _ => None,
        }
    }
}
