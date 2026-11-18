# Localization architecture

OneTagger's desktop window and browser mode render the same Vue/Quasar client.
Translations therefore belong at that shared presentation boundary. The Rust
backend continues to handle files, tagging, provider requests and stable protocol
values independently of the language selected by a client.

Supported application languages are English (`en`), Italian (`it`), French (`fr`),
Dutch (`nl`), German (`de`), Spanish (`es`), Greek (`el`) and Russian (`ru`). Locale
catalogs are UTF-8 JSON under `client/src/locales/<language>/`. Application text
and Quasar's own control labels must switch together. The HTML `lang` attribute
must reflect the selected language for assistive technology. English provides a
fallback for unsupported locales and missing messages.
The application owns HTML language and direction; Quasar's `noHtmlAttrs` setting
prevents a control pack such as `en-US` from replacing the application locale
`en`. All supported languages currently use left-to-right layout.

## Rust boundary and compatibility

There is no universal Rust localization standard requiring a particular crate.
The Rust compiler's own diagnostic guide describes typed message identifiers,
named arguments and catalog validation, but explicitly cautions that its current
Fluent infrastructure is pending redesign and is not mandated even for rustc.
OneTagger follows the separation of message identity, values and presentation
without adding a second translation engine or a process-global locale in Rust.
This also allows different browser clients to render the same event in different
languages safely.

Common backend notifications and errors can carry two optional fields:

```json
{
  "action": "notify",
  "message": "Playlist saved to: /music/collection.m3u",
  "messageKey": "backend.playlistSaved",
  "messageArgs": { "path": "/music/collection.m3u" }
}
```

`crates/onetagger-ui/src/messages.rs` owns these identifiers and argument shapes.
Known errors use a typed Rust error identity, never comparisons against English
error strings. Existing `message` or `error` values remain available verbatim to
older clients. The frontend renders recognized keys with named arguments and
falls back to the original text when a key is unknown. Provider and OS error
details remain ordinary text; they are not interpreted as translation keys,
message templates or HTML.

Auto-tag and Audio Features progress statuses also carry optional `messageKey`
and `messageArgs` fields next to the existing `message`. A typed `TaggingMessage`
helper records the original English text and its descriptor together at the
point where the outcome is known. This covers no-match, already-tagged,
cancellation, album results and file/provider/audio-feature failure explanations.
The error detail remains a literal argument. Old status JSON deserializes without
the new fields, and statuses without descriptors serialize as before. The
presentation metadata is not included in exported tagging reports.

Adding optional serialized fields still changes Rust struct construction: callers
using a `TaggingStatus` literal must initialize `message_key` and `message_args`
to `None` when no descriptor is available. Review new constructors when integrating
other branches. `TaggingStatus` is not part of the custom provider FFI contract.
`TaggerConfig`, `Track`, provider ABI types and the custom platform compatibility
version remain unchanged.

Keep these values independent of locale:

- WebSocket actions/statuses, enum serialization, settings keys, provider IDs and
  plugin ABI values.
- Tag names/IDs used by file formats, music metadata and user-defined tag values,
  including Quick Tag genres and moods.
- File paths, URLs, renamer tokens, profile names, playlist contents, result report
  schemas and machine-readable numeric/date values.
- Diagnostic logging and third-party error details used for troubleshooting.

Only labels describing these values are translated. Display formatting must not
change parsing or storage. Native file dialogs can follow the operating system's
language; this PR does not translate CLI help, installers or arbitrary custom
provider descriptions. Common errors have localized explanations; raw technical
details can remain in their original language.

The built-in help navigation, section headings, workflow summaries and detailed
reference prose are localized. The `help` namespace contains 90 messages in each
of the eight languages, covering provider/API explanations, keyboard walkthroughs,
threshold examples and the renamer reference. Named Vue component slots preserve
links, icons, code examples and interactive elements while allowing translators
to reorder complete sentences. Videos, screenshots and linked external pages
retain their original content.

Built-in provider descriptions, option labels and tooltips are
translated using explicit provider/option IDs, with a strict built-in check so
custom plugins keep their own descriptions. Built-in renamer token documentation
is translated by stable symbol names, while symbol names, format-frame IDs and
executable examples remain unchanged. Unknown/new backend token documentation,
custom provider prose, release changelog text and technical log messages retain
their source language. These are documented coverage boundaries rather than
missing catalog keys.

## Catalog conventions

- Use stable semantic keys scoped by feature and role. English wording is not an
  identifier. When a message's meaning changes, review every translation.
- Translate complete messages, rather than joining translated sentence pieces.
  Use named arguments such as `{path}`, `{detail}` and `{count}` so translators can
  reorder them. Keep technical examples and executable tokens unchanged.
- Use the localization framework's plural support. Russian needs its own plural
  rule; English singular/plural rules do not apply to all requested languages.
  Validate 0, 1, 2, 5, 11, 21, 22, 25 and 101, and fractional values where supported.
- Render translations and substitutions as text. Do not use `v-html` or Quasar
  `html: true` with translations containing filenames, provider responses or user
  data. If markup is needed, keep trusted components in the template and use
  component interpolation so translators can reorder the complete sentence.
- Translation files are bundled with the application, including desktop assets;
  no online translation API or remote font service is needed at runtime.
- Keep locale selection constrained to shipped catalogs, with an English
  fallback for malformed or unsupported preferences. Language selection must not
  affect provider request parameters or library output.

English is the source catalog. Automated validation can establish structural
completeness and rendering behavior; fluent-speaker review remains valuable for
terminology, tone and natural phrasing in the seven translations.

## Verification

All builds and tests run on GitHub. Locale checks should validate matching key
sets, nonempty values, compatible named arguments and valid message syntax.
`ci/i18n_compile.mjs` resolves the actual message compiler through the frozen Vue
I18n dependency tree and compiles every catalog entry in the same JIT mode as the
runtime. Compiler errors fail the Localization workflow before the frontend build;
this does not depend on which messages browser smoke tests happen to display.
Backend regression tests verify additive envelopes, typed error identity, literal
Unicode/path preservation and the presence of every emitted key in all eight
backend catalogs. Existing Rust serialization and file/report tests protect the
machine-readable contracts.

Browser coverage should exercise language selection/persistence/fallback and
normal workflows in the production bundle, including Quasar controls and live
notifications. Layout coverage should include long translated labels, narrow
windows, scrolling and Greek/Cyrillic text. Use stable selectors so translating a
button does not change the test's identity. Retain native desktop smoke coverage
because browser engines and platform font fallback can differ.

## Primary references

- [Rust compiler translation guide](https://rustc-dev-guide.rust-lang.org/diagnostics/translation.html)
- [Vue I18n Composition API](https://vue-i18n.intlify.dev/guide/advanced/composition)
- [Vue I18n type-safe catalogs](https://vue-i18n.intlify.dev/guide/advanced/typescript)
- [Vue I18n pluralization](https://vue-i18n.intlify.dev/guide/essentials/pluralization)
- [Vue I18n fallback behavior](https://vue-i18n.intlify.dev/guide/essentials/fallback)
- [Vue I18n component interpolation](https://vue-i18n.intlify.dev/guide/advanced/component)
- [Vue I18n maintenance status](https://vue-i18n.intlify.dev/guide/maintenance)
- [Pinned Vue I18n compiler export](https://github.com/intlify/vue-i18n/blob/v11.4.22/packages/message-compiler/src/compiler.ts)
- [Pinned Vue I18n runtime compilation](https://github.com/intlify/vue-i18n/blob/v11.4.22/packages/core-base/src/compilation.ts)
- [Quasar language packs](https://quasar.dev/options/quasar-language-packs/)
- [Mozilla localization guidance](https://github.com/mozilla-l10n/documentation/blob/main/src/localization/dev_best_practices.md)
