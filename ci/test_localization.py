"""Catalog contract checks; the Build and Localization workflows run these on GitHub."""
import json
from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[1]
CATALOGS = ROOT / "client/src/locales"
LOCALES = {"en", "it", "fr", "nl", "de", "es", "el", "ru"}
NAMESPACES = {"autotag", "quicktag", "tools", "common", "backend", "core", "providers", "help"}
PARAMETER = re.compile(r"\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*\}")
LITERAL = re.compile(r"\{'(?:\\.|[^'\\])*'\}")
MARKUP = re.compile(r"</?[A-Za-z][A-Za-z0-9:-]*(?:\s[^<>]*?)?/?>")
CONTROL = re.compile(r"[\x00-\x08\x0b\x0c\x0e-\x1f\u202a-\u202e\u2066-\u2069]")
STATIC_KEY = re.compile(r"\b(?:t|translate)\(\s*(['\"])([A-Za-z][\w]*\.[A-Za-z][\w]*)\1")
STATIC_KEYPATH = re.compile(r"\bkeypath\s*=\s*(['\"])([A-Za-z][\w]*\.[A-Za-z][\w]*)\1")


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"Duplicate catalog key: {key}")
        result[key] = value
    return result


def read_catalog(path):
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)


def parameters(message):
    return set(PARAMETER.findall(LITERAL.sub("", message)))


class LocalizationCatalogTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.reference = {path.stem: read_catalog(path) for path in (CATALOGS / "en").glob("*.json")}

    def test_all_requested_locales_and_namespaces_are_present(self):
        self.assertEqual({path.name for path in CATALOGS.iterdir() if path.is_dir()}, LOCALES)
        self.assertTrue(NAMESPACES <= set(self.reference), "Required interface catalog is missing")
        for locale in sorted(LOCALES):
            with self.subTest(locale=locale):
                self.assertEqual({path.stem for path in (CATALOGS / locale).glob("*.json")}, set(self.reference))

    def test_catalogs_have_matching_nonempty_keys_and_safe_named_parameters(self):
        for locale in sorted(LOCALES):
            for namespace, reference in self.reference.items():
                path = CATALOGS / locale / f"{namespace}.json"
                catalog = read_catalog(path)
                with self.subTest(locale=locale, namespace=namespace):
                    self.assertIsInstance(catalog, dict)
                    self.assertEqual(set(catalog), set(reference), "Missing or extra translation key")
                for key, message in catalog.items():
                    with self.subTest(locale=locale, key=f"{namespace}.{key}"):
                        self.assertRegex(key, r"^[A-Za-z][A-Za-z0-9_]*$")
                        self.assertIsInstance(message, str, "Messages must be flat string values")
                        self.assertTrue(message.strip(), "Empty translation")
                        self.assertNotRegex(message, MARKUP, "Render rich content as Vue components, not catalog HTML")
                        self.assertNotRegex(message, CONTROL, "Unexpected control or bidi override character")
                        stripped = LITERAL.sub("", message)
                        self.assertEqual(parameters(message), parameters(reference[key]), "Named placeholder mismatch")
                        syntax = PARAMETER.sub("", stripped)
                        self.assertNotIn("{", syntax, "Escape literal braces with Vue I18n literal interpolation")
                        self.assertNotIn("}", syntax, "Unbalanced interpolation")
                        self.assertNotIn("@:", stripped, "Catalog links are not part of this translation contract")
                        if "|" in LITERAL.sub("", reference[key]):
                            self.assertGreaterEqual(len(stripped.split("|")), 2, "Plural alternatives were lost")

    def test_locales_are_translated_and_greek_cyrillic_are_preserved(self):
        for locale in sorted(LOCALES - {"en"}):
            catalogs = {path.stem: read_catalog(path) for path in (CATALOGS / locale).glob("*.json")}
            translated = sum(value != self.reference[namespace][key]
                             for namespace, catalog in catalogs.items() for key, value in catalog.items())
            with self.subTest(locale=locale):
                self.assertGreater(translated, 0, "Locale only duplicates the English fallback")
                text = " ".join(value for catalog in catalogs.values() for value in catalog.values())
                if locale == "el":
                    self.assertRegex(text, r"[\u0370-\u03ff\u1f00-\u1fff]", "Greek glyphs are missing")
                if locale == "ru":
                    self.assertRegex(text, r"[\u0400-\u04ff]", "Cyrillic glyphs are missing")

    def test_static_translation_keys_exist_and_translations_are_not_html(self):
        keys = {f"{namespace}.{key}" for namespace, catalog in self.reference.items() for key in catalog}
        for path in (ROOT / "client/src").rglob("*"):
            if path.suffix not in {".ts", ".vue"}:
                continue
            source = path.read_text(encoding="utf-8")
            for _, key in STATIC_KEY.findall(source) + STATIC_KEYPATH.findall(source):
                with self.subTest(file=str(path.relative_to(ROOT)), key=key):
                    self.assertIn(key, keys, "Translation reference has no English fallback")
            for attribute in re.findall(r'''v-html\s*=\s*("[^"]*"|'[^']*')''', source):
                self.assertNotRegex(attribute, r"(?:\bt|\$t|translate)\s*\(",
                                    f"Do not render a translated message as HTML: {path.relative_to(ROOT)}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
