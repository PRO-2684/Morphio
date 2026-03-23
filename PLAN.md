# Text Morph Implementation Plan

## Goal

Implement the `TextMorph` trait so a font can be patched to render one word as another word of the same length, following the behavior demonstrated in `doc/ref.py`.

Out of scope for this pass:

- WASM support
- polishing the CLI or demo binary
- broad feature-file compatibility beyond the specific substitution pattern we need

## Target Behavior

For `morph("worda", "wordb")`, the library should:

1. Validate both words.
2. Resolve each character to a glyph through the font's best Unicode `cmap`.
3. Create or update a `GSUB` table that applies contextual substitutions only when the full source word appears.
4. Prevent the substitution when the source word is embedded inside a larger alphabetic word, matching the Python reference's `ignore substitute` guards.
5. Rebuild the font with the modified `GSUB` and preserve the other tables.

## Design Direction

### 1. Clarify the crate API

- Decide which concrete font-owning type implements `TextMorph`.
- Keep the public trait small; move table-building details into helper functions or extension traits.
- Make `MorphError` carry real source errors instead of collapsing everything to `ReadError` / `BuilderError`.

Likely result:

- `TextMorph` implemented for an owned font wrapper, not directly for `read_fonts::FontRef<'_>`.
- `src/lib.rs` exposes the high-level operation and error types.
- `src/ext/` contains lower-level helpers for `FontRef`, `Gsub`, and possibly glyph/class utilities.

### 2. Build the minimal GSUB model we actually need

The Python reference uses two GSUB concepts:

- reusable single substitutions for each distinct `src -> dst` glyph pair
- one contextual substitution rule that triggers those lookups across the full source word

In `write-fonts`, we should mirror that directly instead of trying to compile `.fea` text.

Implementation pieces:

- Ensure we can read an existing `GSUB`, or create a new one if the font does not already have one.
- Add `SingleSubst` lookups for every unique glyph replacement where source and target glyph differ.
- Add one chaining contextual substitution lookup that:
  - matches the full source glyph sequence
  - invokes the per-position single-substitution lookups
  - includes boundary guards equivalent to the Python `ignore substitute` rules

### 3. Define boundary behavior explicitly

The reference script treats ASCII letters as "word characters" via `@LETTER`.

We need to decide whether the Rust crate should:

- match the Python reference exactly with an ASCII-letter class for the first implementation, or
- generalize the boundary class based on Unicode or font coverage

Recommended first step:

- implement the same ASCII-letter behavior as `doc/ref.py`
- isolate the boundary-class construction so it can be expanded later without changing the public API

## Execution Plan

### Phase 1: Establish font mutation primitives

- Introduce an owned font type that can:
  - read bytes into `FontRef`
  - own / rebuild modified tables with `FontBuilder`
  - emit patched font bytes
- Wire `MorphError` conversions from `read-fonts` and `write-fonts`.
- Keep `src/main.rs` as disposable scratch code or remove it once the library path is proven.

### Phase 2: Glyph resolution and validation

- Reuse `FontRefExt::best_cmap()`.
- Add helper(s) to map a Rust `char` sequence to glyph IDs.
- Validate:
  - equal string length by character count
  - non-empty source and target words
  - all characters are present in the chosen `cmap`

Open question:

- whether empty strings should be rejected explicitly; the Python reference rejects them, and the Rust crate should likely do the same

### Phase 3: GSUB lookup construction

- Add helpers in `src/ext/gsub.rs` for:
  - appending a lookup and returning its index
  - creating a `SingleSubst` lookup for one glyph pair
  - creating a chaining contextual substitution lookup
- Model lookup indices carefully; the contextual rule must reference the lookup list indices of the single-substitution lookups.
- Reuse identical `src -> dst` substitutions instead of emitting duplicate lookups.

### Phase 4: Boundary guards

- Construct the equivalent of:
  - `ignore substitute @LETTER <word>;`
  - `ignore substitute <word> @LETTER;`
- Confirm which `write-fonts` layout structures correspond to ignore rules.
- If `ignore substitute` is awkward to encode directly, verify whether the same behavior can be expressed with explicit backtrack/lookahead coverage and separate contextual subtables.

This is the riskiest implementation area and should be validated early with a small real font.

### Phase 5: Integrate into `TextMorph`

- Read `GSUB` from the source font or initialize a minimal one.
- Append the new lookups and, if needed, add a `FeatureRecord` / `ScriptList` entry for `calt`.
- Rebuild the font bytes and return them through the owned wrapper.
- Preserve existing tables and avoid destroying unrelated layout data.

### Phase 6: Testing and verification

- Add unit tests for:
  - length mismatch
  - empty input
  - missing glyph
  - glyph-pair deduplication
- Add integration tests with a real font fixture to confirm:
  - the patched font builds successfully
  - the resulting font contains a `GSUB` table
  - the expected lookups / features are present
- If feasible, add a golden-style comparison against the Python-generated feature logic for a simple case such as `abc -> xyz`.

## Risks and Unknowns

- `write-fonts` layout APIs are low-level; wiring `ScriptList`, `FeatureList`, `LookupList`, and lookup indices correctly may take iteration.
- Fonts without an existing `GSUB` table need a clean minimal initialization path.
- The current docs and Python script use feature-file semantics; not every concept maps obviously to a single Rust helper type.
- Character count and glyph coverage are not the same thing for non-ASCII text; we should keep the first implementation conservative.

## Recommended Order of Work

1. Define the owned-font API and error model.
2. Implement glyph resolution and validation.
3. Prove basic `SingleSubst` lookup insertion on a test font.
4. Prove contextual lookup insertion for a fixed word.
5. Add boundary guards.
6. Hook everything into `TextMorph::morph`.
7. Remove scratch code and tighten tests/docs.

## Deliverable for the first implementation

The first complete version should:

- patch a non-WASM font in memory
- support one `from_word -> to_word` mapping per call
- preserve unrelated tables
- follow the Python reference for ASCII word boundaries
- ship with tests that exercise both error paths and one successful font rewrite
