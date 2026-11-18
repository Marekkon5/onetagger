"""Localized production-frontend regressions, run only on GitHub runners.

The HTTP/WebSocket fixture is shared with ui_smoke.py. These tests exercise the
browser UI; the normal Build workflow covers Rust and native integration.
"""
from contextlib import contextmanager
import json
import os
import re
import time
import unittest

from playwright.sync_api import expect
from ui_smoke import Backend, BrowserTests, ROOT, TOKEN, TRACK


RESULTS = ROOT / "localization-results"
LOCALES = ("en", "it", "fr", "nl", "de", "es", "el", "ru")
NATIVE_NAMES = {"en": "English", "it": "Italiano", "fr": "Français", "nl": "Nederlands",
                "de": "Deutsch", "es": "Español", "el": "Ελληνικά", "ru": "Русский"}
# Exact defaults from the pinned Quasar v2.35.4 ui/lang/{en-US,fr,ru}.js.
# OK is identical in these packs; Cancel is what proves the pack changes.
QUASAR_CANCEL = {"en": "Cancel", "fr": "Annuler", "ru": "Отмена"}
RAW_KEY = re.compile(r"\b(?:autotag|quicktag|tools|common|backend|core)\.[A-Za-z][A-Za-z0-9_]*\b")


def message(locale, namespace, key):
    path = ROOT / "client/src/locales" / locale / f"{namespace}.json"
    return json.loads(path.read_text(encoding="utf-8"))[key]


def selection(count=1):
    return {"schema": 1, "id": "a" * 32, "snapshot": "synthetic", "snapshot_digest": "b" * 64,
            "scope_key": "capture:fixture", "mapping_revision": "c" * 64,
            "tracks": [{"track_id": f"library-{index}",
                        "local_path": f"/music/{index} Café Ελληνικά Русский 100%20.mp3"}
                       for index in range(count)]}


class LocalizedBackend(Backend):
    """Extend the shared fixture with empty Quick Tag and Renamer responses."""
    def websocket(self, socket):
        self.socket = socket
        owner = self

        class Proxy:
            def __getattr__(self, name):
                return getattr(socket, name)

            def on_message(self, callback):
                def receive(raw):
                    data = json.loads(raw)
                    action = data["action"]
                    if action == "quickTagFolder":
                        reply = {"action": action, "path": data.get("path") or "/music", "files": []}
                    elif action == "quickTagLoad":
                        reply = {"action": action, "data": {"files": [], "failed": []}}
                    elif action == "renamerSyntaxHighlight":
                        reply = {"action": action, "html": ""}
                    elif action == "renamerAutocomplete":
                        reply = {"action": action, "suggestions": [], "offset": 0}
                    else:
                        callback(raw)
                        return
                    owner.commands.append(data)
                    socket.send(json.dumps(reply))
                socket.on_message(receive)

        super().websocket(Proxy())


class LocalizationTests(unittest.TestCase):
    engine = "chromium"

    @classmethod
    def setUpClass(cls):
        BrowserTests.setUpClass.__func__(cls)
        RESULTS.mkdir(exist_ok=True)

    @classmethod
    def tearDownClass(cls):
        BrowserTests.tearDownClass.__func__(cls)

    @contextmanager
    def session(self, browser_locale="en-US", saved=None, expected="en", suffix="",
                viewport=None, status=204):
        context = self.browser.new_context(viewport=viewport or {"width": 1440, "height": 900},
                                          locale=browser_locale, timezone_id="UTC", service_workers="block")
        if saved is not None:
            context.add_init_script(f"localStorage.setItem('onetagger.locale', {json.dumps(saved)});")
        context.tracing.start(screenshots=True, snapshots=True, sources=False)
        page = context.new_page()
        page.set_default_timeout(10000)
        backend = LocalizedBackend(self.base_url, status)
        errors = []
        warnings = []
        page.on("pageerror", lambda error: errors.append(str(error)))
        page.on("console", lambda entry: warnings.append(entry.text)
                if entry.type in {"warning", "error"} and "[intlify]" in entry.text else None)
        page.route("**/*", backend.http)
        page.route_web_socket("**/ws", backend.websocket)
        output = RESULTS / f"{self.engine}-{self._testMethodName}-{suffix or expected}"
        try:
            page.goto(self.base_url + "/#session=" + TOKEN)
            expect(page.locator("html")).to_have_attribute("lang", expected)
            if status == 204:
                expect(page.get_by_test_id("nav-home")).to_be_visible()
                # Clicking navigation also waits for the platform-loading dialog
                # to close, independently of its translated caption.
                page.get_by_test_id("nav-home").click()
            yield page, backend
            self.assertEqual(errors, [], "Browser JavaScript exceptions")
            self.assertEqual(warnings, [], "Missing keys or invalid message compilation")
            self.assertEqual(backend.unexpected_requests, [], "Unexpected external network request")
            self.assertEqual(backend.unexpected_commands, [], "Fixture protocol needs explicit coverage")
            page.screenshot(path=str(output.with_suffix(".png")), full_page=True, animations="disabled")
        except Exception:
            page.screenshot(path=str(output.with_suffix(".png")), full_page=True, animations="disabled")
            context.tracing.stop(path=str(output.with_suffix(".zip")))
            raise
        else:
            context.tracing.stop()
        finally:
            context.close()

    def choose_locale(self, page, locale):
        selector = page.get_by_test_id("language-selector").first
        expect(selector).to_be_visible()
        selector.click()
        page.get_by_role("option", name=NATIVE_NAMES[locale], exact=True).click()
        expect(page.locator("html")).to_have_attribute("lang", locale)
        self.assertEqual(page.evaluate("localStorage.getItem('onetagger.locale')"), locale)

    def open_autotag_input(self, page):
        page.get_by_test_id("nav-autotagger").click()
        expect(page.get_by_role("checkbox", name="Beatport", exact=True)).to_be_visible()
        page.locator(".q-stepper__tab").nth(1).click()
        expect(page.get_by_test_id("helper-selection")).to_have_count(1)

    def upload_selection(self, page, value):
        page.locator('[data-testid="helper-selection"] input[type=file], '
                     'input[type=file][data-testid="helper-selection"]').set_input_files({
            "name": "selection.json", "mimeType": "application/json",
            "buffer": json.dumps(value, ensure_ascii=False).encode("utf-8")})
        expect(page.get_by_test_id("helper-selection-count")).to_be_visible()

    def assert_layout(self, page, width):
        self.assertLessEqual(page.evaluate("document.documentElement.scrollWidth"), width + 1,
                             "Translation introduced horizontal page overflow")
        self.assertNotRegex(page.locator(".q-layout").inner_text(), RAW_KEY,
                            "Untranslated message key leaked into the interface")
        page.evaluate("document.fonts.ready")

    def finish_selected_tagging(self, page, backend):
        previous = len([c for c in backend.commands if c["action"] == "startTagging"])
        backend.hold_tagging = True
        page.get_by_test_id("autotagger-start").click()
        expect(page).to_have_url(re.compile(r"#/autotagger/status$"))
        expect(page.locator(".progress")).to_be_visible()
        deadline = time.monotonic() + 10
        while len([c for c in backend.commands if c["action"] == "startTagging"]) == previous:
            if time.monotonic() >= deadline:
                self.fail("Auto-tag did not send its selection to the backend")
            page.wait_for_timeout(20)
        backend.tagging_socket.send(json.dumps(backend.tagging_done()))
        backend.hold_tagging = False
        dialog = page.get_by_role("dialog")
        expect(dialog).to_be_visible()
        return dialog

    def test_all_languages_preserve_autotag_and_editor_payloads(self):
        for locale in LOCALES:
            with self.subTest(locale=locale), self.session(saved=locale, expected=locale) as (page, backend):
                self.open_autotag_input(page)
                chosen = selection()
                self.upload_selection(page, chosen)
                dialog = self.finish_selected_tagging(page, backend)
                if locale in QUASAR_CANCEL:
                    expect(dialog.get_by_role("button", name=QUASAR_CANCEL[locale], exact=True)).to_be_visible()
                dialog.get_by_role("button").first.click()
                command = next(c for c in backend.commands if c["action"] == "startTagging")
                self.assertEqual(command["selection"], chosen)
                self.assertTrue(command["report"])
                self.assertIsNone(command["playlist"])
                self.assertEqual(command["config"]["platforms"], ["beatport"])

                page.get_by_test_id("nav-tageditor").click()
                page.locator("#fileList").get_by_text("Synthetic track.mp3", exact=True).click()
                title = page.locator('[data-testid="tag-field-TIT2"] input, '
                                     'input[data-testid="tag-field-TIT2"]')
                expect(title).to_have_value("Synthetic title")
                raw = 'Café Ελληνικά Русский <img src=x onerror="window.i18nInjected=true">'
                title.fill(raw)
                title.press("Tab")
                page.get_by_test_id("tag-editor-save").click()
                expect(page.get_by_text(message(locale, "tools", "tagsWritten"), exact=True)).to_be_visible()
                save = backend.saves()[-1]
                self.assertEqual(save["path"], TRACK["path"])
                self.assertIn({"type": "raw", "tag": "TIT2", "value": [raw]}, save["changes"])
                self.assertIsNone(page.evaluate("window.i18nInjected"))
                expect(title).to_have_value(raw)
                self.assert_layout(page, 1440)

    def test_switching_updates_current_screen_and_survives_reload(self):
        with self.session() as (page, backend):
            before = page.get_by_test_id("nav-tageditor").inner_text()
            self.choose_locale(page, "it")
            expect(page.get_by_test_id("nav-tageditor")).not_to_have_text(before)
            page.reload()
            expect(page.locator("html")).to_have_attribute("lang", "it")
            expect(page.get_by_test_id("nav-home")).to_be_visible()
            self.choose_locale(page, "ru")
            self.assertRegex(page.get_by_test_id("nav-tageditor").inner_text(), r"[\u0400-\u04ff]")
            for locale in ("ru", "fr", "en"):
                self.choose_locale(page, locale)
                self.open_autotag_input(page)
                self.upload_selection(page, selection())
                dialog = self.finish_selected_tagging(page, backend)
                cancel = dialog.get_by_role("button", name=QUASAR_CANCEL[locale], exact=True)
                expect(cancel).to_be_visible()
                cancel.click()
                backend.socket.send(json.dumps({"action": "error", "message": "Quasar default button fixture"}))
                # This error dialog has no custom OK label, unlike Auto-tag.
                default_ok = page.get_by_role("dialog").get_by_role("button", name="OK", exact=True)
                expect(default_ok).to_be_visible()
                default_ok.click()
            self.choose_locale(page, "el")
            self.assertRegex(page.get_by_test_id("nav-tageditor").inner_text(), r"[\u0370-\u03ff]")
            self.choose_locale(page, "en")
            expect(page.get_by_test_id("nav-tageditor")).to_have_text(before)

    def test_regional_detection_and_unsupported_saved_language_fallback(self):
        for browser_locale, saved, expected in [("fr-CH", None, "fr"), ("de-CH", None, "de"),
                                                ("ja-JP", None, "en"), ("fr-CH", "unsupported", "en")]:
            with self.subTest(browser_locale=browser_locale, saved=saved), self.session(
                    browser_locale=browser_locale, saved=saved, expected=expected,
                    suffix=f"{browser_locale}-{saved or 'auto'}") as (page, _):
                self.assert_layout(page, 1440)

    def test_clients_can_use_different_languages_without_changing_backend_state(self):
        with self.session(saved="fr", expected="fr") as (french, french_backend), self.session(
                saved="de", expected="de") as (german, german_backend):
            self.choose_locale(french, "it")
            expect(german.locator("html")).to_have_attribute("lang", "de")
            expect(german.get_by_test_id("nav-tageditor")).to_contain_text(message("de", "common", "editTags"))
            for backend in (french_backend, german_backend):
                self.assertFalse(any(command["action"] in {"setLocale", "setLanguage"} for command in backend.commands))

    def test_russian_plural_forms_are_rendered_by_the_actual_application(self):
        with self.session(saved="ru", expected="ru") as (page, _):
            self.open_autotag_input(page)
            forms = [form.strip() for form in message("ru", "tools", "selectedTracks").split("|")]
            self.assertGreaterEqual(len(forms), 3)
            for count, index in [(1, 0), (2, 1), (5, 2), (11, 2), (21, 0), (22, 1), (25, 2)]:
                self.upload_selection(page, selection(count))
                text = forms[index].replace("{count}", str(count))
                expect(page.get_by_test_id("helper-selection-count")).to_contain_text(text)

    def test_backend_keys_fallback_and_untrusted_arguments_render_as_text(self):
        with self.session(saved="fr", expected="fr") as (page, backend):
            detail = '<img src=x onerror="window.i18nInjected=true"> provider & file'
            backend.socket.send(json.dumps({"action": "error", "message": "Legacy fallback",
                                             "messageKey": "backend.requestFailed", "messageArgs": {"detail": detail}}))
            dialog = page.get_by_role("dialog")
            expect(dialog.get_by_role("button", name="OK", exact=True)).to_be_visible()
            expect(dialog).to_contain_text(detail)
            self.assertIsNone(page.evaluate("window.i18nInjected"))
            expect(dialog.locator("img")).to_have_count(0)
            expect(dialog).not_to_contain_text("Legacy fallback")
            dialog.get_by_role("button").first.click()
            fallback = "Unknown provider detail <raw>"
            backend.socket.send(json.dumps({"action": "error", "message": fallback,
                                             "messageKey": "backend.futureMessage", "messageArgs": {}}))
            expect(page.get_by_role("dialog")).to_contain_text(fallback)
            expect(page.get_by_role("dialog").locator("raw")).to_have_count(0)
            page.get_by_role("dialog").get_by_role("button").first.click()
            malformed = "Older backend message remains available"
            backend.socket.send(json.dumps({"action": "error", "message": malformed,
                                             "messageKey": "backend.requestFailed",
                                             "messageArgs": {"detail": {"unexpected": "object"}}}))
            expect(page.get_by_role("dialog")).to_contain_text(malformed)
            page.get_by_role("dialog").get_by_role("button").first.click()
            backend.socket.send(json.dumps({"action": "error", "message": "Legacy fallback",
                                             "messageKey": "backend.requestFailed", "messageArgs": {
                                                 "detail": "Prototype guard", "__proto__": {"i18nPolluted": True}}}))
            expect(page.get_by_role("dialog")).to_contain_text("Prototype guard")
            self.assertIsNone(page.evaluate("({}).i18nPolluted"))
            page.get_by_role("dialog").get_by_role("button").first.click()

    def test_longer_languages_fit_and_all_main_screens_remain_navigable(self):
        for locale in ("de", "nl", "el", "ru"):
            with self.subTest(locale=locale), self.session(saved=locale, expected=locale,
                    viewport={"width": 1280, "height": 720}) as (page, _):
                for route in ("autotagger", "tageditor", "renamer", "audiofeatures", "quicktag", "home"):
                    page.get_by_test_id(f"nav-{route}").click()
                    expect(page.locator(".q-page-container")).not_to_be_empty()
                    self.assert_layout(page, 1280)
                page.set_viewport_size({"width": 1100, "height": 700})
                warning = page.get_by_role("dialog").filter(has_text="1200")
                expect(warning).to_be_visible()
                page.set_viewport_size({"width": 1280, "height": 720})
                expect(warning).to_have_count(0)
                self.assert_layout(page, 1280)


if __name__ == "__main__":
    failed = False
    summary = []
    for engine in os.environ.get("UI_BROWSERS", "chromium,firefox,webkit").split(","):
        if engine not in {"chromium", "firefox", "webkit"}:
            raise ValueError("Unknown browser engine")
        LocalizationTests.engine = engine
        print(f"\nLocalization browser engine: {engine}", flush=True)
        result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(LocalizationTests))
        summary.append({"browser": engine, "locales": list(LOCALES), "tests": result.testsRun,
                        "failures": len(result.failures), "errors": len(result.errors)})
        failed |= not result.wasSuccessful()
    RESULTS.mkdir(exist_ok=True)
    (RESULTS / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as output:
            output.write("### Localization regressions\n\n| Engine | Locales | Tests | Failures | Errors |\n|---|---:|---:|---:|---:|\n")
            for row in summary:
                output.write(f"| {row['browser']} | {len(row['locales'])} | {row['tests']} | {row['failures']} | {row['errors']} |\n")
            output.write("\nProduction frontend with synthetic HTTP/WebSocket data. No real provider calls or music files; "
                         "native WebViews and Rust protocol compatibility are covered separately by Build.\n")
    raise SystemExit(1 if failed else 0)
