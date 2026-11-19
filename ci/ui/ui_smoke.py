"""Browser regressions against the built Vue UI with a synthetic WebSocket backend.

Run on GitHub: python ci/ui/ui_smoke.py. No Rust backend, real music, provider
credentials or external network services are needed. This is not a native
WebView or backend authorization test; Rust tests cover the production routes.
"""
from contextlib import contextmanager
from copy import deepcopy
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import threading
import unittest
from urllib.parse import urlsplit

from playwright.sync_api import expect, sync_playwright

ROOT = Path(__file__).resolve().parents[2]
RESULTS = ROOT / "ui-test-results"
TOKEN = "a" * 64  # Synthetic capability, never a real application session.
PNG = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a1XcAAAAASUVORK5CYII="
PLATFORMS = [{"builtIn": True, "requiresAuth": False, "supportedTags": ["title", "genre"], "icon": PNG,
              "platform": {"id": identity, "name": name, "description": "Offline fixture", "version": "1",
                           "maxThreads": 1, "customOptions": {"options": []}}}
             for identity, name in [("beatport", "Beatport"), ("traxsource", "Traxsource")]]
TRACK = {
    "filename": "Synthetic track.mp3", "path": "/music/Synthetic track.mp3", "format": "mp3",
    "tags": {"TIT2": "Synthetic title", "TPE1": "Test artist"},
    "images": [{"mime": "image/png", "data": PNG, "kind": kind,
                "description": "Synthetic artwork", "width": 1, "height": 1}
               for kind in ["CoverFront", "CoverBack"]],
    "id3": {"comments": [{"lang": "eng", "description": "test", "text": "Synthetic comment"}],
            "unsync_lyrics": [{"lang": "eng", "description": "test", "text": "Synthetic lyrics"}],
            "popularimeter": None},
}


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, *_):
        pass


class Backend:
    """Fixture protocol only. Outgoing commands are recorded for assertions."""
    def __init__(self, base_url, session_status=204, autosave=False):
        self.base_url = base_url
        self.session_status = session_status
        self.autosave = autosave
        self.save_failures = 0
        self.hold_saves = False
        self.pending_saves = []
        self.commands = []
        self.sessions = []
        self.connections = 0
        self.unexpected_requests = []
        self.unexpected_commands = []
        self.hold_tagging = False
        self.tagging_socket = None
        self.reject_stop = False

    def http(self, route):
        request = route.request
        if request.url == self.base_url + "/session":
            self.sessions.append({"method": request.method, "body": request.post_data})
            route.fulfill(status=self.session_status, body="")
        elif request.url == "https://api.github.com/repos/fantuz/onetagger/releases/latest":
            route.fulfill(json={"tag_name": "v" + (ROOT / "VERSION").read_text().strip(),
                                "body": "Synthetic release", "html_url": "https://example.invalid/release"})
        elif request.url.startswith(self.base_url + "/"):
            route.continue_()
        else:
            self.unexpected_requests.append(request.url)
            route.abort()

    def websocket(self, ws):
        self.connections += 1

        def message(raw):
            data = json.loads(raw)
            self.commands.append(data)
            action = data["action"]
            response = None
            if action == "init":
                response = {"action": action, "version": (ROOT / "VERSION").read_text().strip(),
                            "os": "linux", "arch": "x86_64", "customPlatformCompat": 1,
                            "commit": "ui-fixture", "workDir": "/music", "dataDir": "/synthetic",
                            "renamerDocs": {}, "startContext": {"serverMode": False,
                            "expose": False, "browser": True, "startPath": "/music"}}
            elif action == "loadPlatforms":
                response = {"action": action, "platforms": deepcopy(PLATFORMS)}
            elif action == "startTagging":
                ws.send(json.dumps({"action":action,"files":len((data.get('selection') or {}).get('tracks',[])),"type":"autoTagger"}))
                self.tagging_socket = ws
                if not self.hold_tagging:
                    response = self.tagging_done()
            elif action == "stopTagging":
                response = ({"action":"error","message":"Simulated Stop failure"} if self.reject_stop else
                            {"action":"stopTagging","accepted":True})
            elif action == "defaultCustomPlatformSettings":
                response = {"action": action, "custom": {}}
            elif action == "spotifyAuthorized":
                response = {"action": action, "value": False}
            elif action == "tagEditorFolder":
                response = {"action": action, "path": "/music", "recursive": False, "files": [
                    {"filename": name, "path": "/music/" + name, "dir": False, "playlist": False}
                    for name in ["Synthetic track.mp3", "Another track.flac"]]}
            elif action == "tagEditorLoad":
                track = deepcopy(TRACK)
                track["path"] = data["path"]
                track["filename"] = data["path"].rsplit("/", 1)[-1]
                response = {"action": action, "data": track}
            elif action == "tagEditorSave":
                if self.hold_saves:
                    self.pending_saves.append((ws, data))
                else:
                    ok = self.save_failures == 0
                    self.save_failures = max(0, self.save_failures - 1)
                    response = self.save_response(data, ok)
            elif action == "loadSettings" and self.autosave:
                # These nested fields are required by the saved-settings decoder.
                # An empty quickTag object throws before autosave can be restored.
                response = {"action": action, "settings": {
                    "path": "/music", "tagEditorAutosave": True, "autoTaggerConfig": {},
                    "audioFeatures": {}, "renamer": {}, "quickTag": {
                        "noteTag": {"tag": {"id3": "COMM", "vorbis": "COMMENT", "mp4": "©cmt"}},
                        "moodTag": {"id3": "TMOO", "vorbis": "MOOD", "mp4": "iTunes:MOOD"},
                        "energyTag": {"tag": {"id3": "TCOM", "vorbis": "COMPOSER", "mp4": "©wrt"}},
                        "energyKeys": [None] * 5}}}
            elif action not in {"loadSettings", "saveSettings", "playerVolume", "playerStop"}:
                self.unexpected_commands.append(action)
            # A first-run backend may have no saved settings; preserve actual UI defaults.
            if response is not None:
                ws.send(json.dumps(response))

        ws.on_message(message)

    @staticmethod
    def save_response(data, ok=True):
        return {"action": "tagEditorSave", "requestId": data["requestId"],
                "path": data["changes"]["path"], "status": "ok" if ok else "error",
                "message": None if ok else "Simulated write permission failure"}

    def acknowledge_next_save(self, ok=True):
        ws, data = self.pending_saves.pop(0)
        ws.send(json.dumps(self.save_response(data, ok)))

    def saves(self):
        return [command["changes"] for command in self.commands if command["action"] == "tagEditorSave"]

    @staticmethod
    def tagging_done(cancelled=False):
        return {"action":"taggingDone","path":None,"cancelled":cancelled,"data":{"successFile":"/synthetic/success.m3u",
                "failedFile":"/synthetic/failed.m3u","moveErrors":[],"reportError":None,"reportFile":"/synthetic/results.json"}}

    def tagging_progress(self,path='/music/in-flight.mp3'):
        self.tagging_socket.send(json.dumps({"action":"taggingProgress","status":{"platform":"beatport","progress":0.25,
            "status":{"path":path,"status":"ok","accuracy":1.0,"usedShazam":False}}}))


class BrowserTests(unittest.TestCase):
    engine = "chromium"

    @classmethod
    def setUpClass(cls):
        if not (ROOT / "client/dist/index.html").is_file():
            raise RuntimeError("Build/download the production frontend artifact before running UI tests")
        RESULTS.mkdir(exist_ok=True)
        cls.server = ThreadingHTTPServer(("127.0.0.1", 0), partial(QuietHandler, directory=str(ROOT / "client/dist")))
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()
        cls.base_url = f"http://127.0.0.1:{cls.server.server_port}"
        cls.playwright = sync_playwright().start()
        cls.browser = getattr(cls.playwright, cls.engine).launch()

    @classmethod
    def tearDownClass(cls):
        cls.browser.close()
        cls.playwright.stop()
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join(timeout=5)

    @contextmanager
    def session(self, status=204, fragment=None, autosave=False):
        context = self.browser.new_context(viewport={"width": 1440, "height": 900},
                                           locale="en-US", timezone_id="UTC", service_workers="block")
        context.tracing.start(screenshots=True, snapshots=True, sources=False)
        page = context.new_page()
        page.set_default_timeout(10000)
        backend = Backend(self.base_url, status, autosave)
        errors = []
        page.on("pageerror", lambda error: errors.append(str(error)))
        page.route("**/*", backend.http)
        page.route_web_socket("**/ws", backend.websocket)
        output = RESULTS / f"{self.engine}-{self._testMethodName}"
        try:
            page.goto(self.base_url + "/" + (fragment if fragment is not None else "#session=" + TOKEN))
            if status == 204:
                expect(page.get_by_text("The ultimate cross-platform music tagger for DJs", exact=True)).to_be_visible()
                expect(page.get_by_text("Loading platforms...", exact=True)).not_to_be_visible()
            yield page, backend
            self.assertEqual(errors, [], "Browser JavaScript exceptions")
            self.assertEqual(backend.unexpected_requests, [], "Unexpected external network requests")
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

    def test_autotagger_short_panels_collapse_after_scrolling_and_resize(self):
        with self.session() as (page, _):
            page.get_by_role('tab', name='Auto tag', exact=True).click()
            for width, height in [(1200, 550), (1280, 720), (1920, 1080)]:
                with self.subTest(width=width, height=height):
                    page.set_viewport_size({'width': width, 'height': height})
                    for _cycle in range(2):
                        page.get_by_text('Advanced', exact=True).click()
                        expect(page.get_by_text('Miscellaneous options', exact=True)).to_be_visible()
                        # Exercise real wheel scrolling, not just DOM assignment.
                        page.mouse.move(width // 2, height // 2)
                        page.mouse.wheel(0, 10000)
                        page.wait_for_function("document.querySelector('.content').scrollTop > 100")
                        page.get_by_label('Cover filename template', exact=True).scroll_into_view_if_needed()
                        expect(page.get_by_label('Cover filename template', exact=True)).to_be_visible()
                        page.get_by_text('Platform Specific Settings', exact=True).click()
                        message = page.get_by_text('No platform specific settings available for the selected platform(s)', exact=True)
                        expect(message).to_be_visible()
                        page.wait_for_function("document.querySelector('.content').scrollTop === 0")
                        geometry = page.evaluate("""() => {
                            const panel = document.querySelector('.content');
                            const inner = document.querySelector('.q-stepper__step-inner');
                            const step = inner.closest('.q-stepper__step');
                            const bar = document.querySelector('.at-stepper-bar');
                            return {
                                unused: step.getBoundingClientRect().height - inner.getBoundingClientRect().height,
                                stepHeight: step.getBoundingClientRect().height,
                                excessScroll: panel.scrollHeight - panel.clientHeight,
                                documentScroll: document.documentElement.scrollHeight - innerHeight,
                                width: document.documentElement.scrollWidth,
                                barBottom: bar.getBoundingClientRect().bottom,
                            };
                        }""")
                        self.assertLessEqual(geometry['unused'], 2, geometry)
                        self.assertLess(geometry['stepHeight'], 220, geometry)
                        self.assertLessEqual(geometry['excessScroll'], 2, geometry)
                        self.assertLessEqual(geometry['documentScroll'], 2, geometry)
                        self.assertLessEqual(geometry['width'], width, geometry)
                        self.assertAlmostEqual(geometry['barBottom'], height, delta=2)
                    page.screenshot(path=str(RESULTS / f'{self.engine}-collapsed-{width}x{height}.png'))
                    # Returning from another route must start at the panel top.
                    page.get_by_role('tab', name='Home', exact=True).click()
                    expect(page.get_by_text('The ultimate cross-platform music tagger for DJs', exact=True)).to_be_visible()
                    page.get_by_role('tab', name='Auto tag', exact=True).click()
                    expect(page.get_by_role('checkbox', name='Beatport', exact=True)).to_be_visible()
                    page.wait_for_function("document.querySelector('.content').scrollTop === 0")

    def test_autotagger_hidden_advanced_options_release_their_space(self):
        with self.session() as (page, _):
            page.get_by_role('tab', name='Auto tag', exact=True).click()
            page.get_by_text('Advanced', exact=True).click()
            toggle = page.get_by_role('switch', name='Parse metadata from filename', exact=True)
            # Quasar QToggle exposes the switch role; assert both control and layout.
            toggle.scroll_into_view_if_needed()
            if toggle.get_attribute('aria-checked') == 'true':
                toggle.click()
            field = page.get_by_label('Filename scheme', exact=True)
            expect(field).to_have_count(0)
            panel = page.locator('.q-stepper__step-inner')
            collapsed = panel.bounding_box()['height']
            toggle.click()
            expect(field).to_be_visible()
            expanded = panel.bounding_box()['height']
            self.assertGreater(expanded, collapsed + 20)
            toggle.click()
            expect(field).to_have_count(0)
            self.assertAlmostEqual(panel.bounding_box()['height'], collapsed, delta=2)

    def test_autotagger_checkbox_mouse_and_keyboard(self):
        with self.session() as (page, _backend):
            def exercise(control):
                initial = control.get_attribute('aria-checked')
                toggled = 'false' if initial == 'true' else 'true'
                control.click()
                expect(control).to_have_attribute('aria-checked', toggled)
                # Mouse interaction must leave the actual checkbox keyboard-
                # accessible, including Space and Enter.
                control.focus()
                expect(control).to_be_focused()
                control.press('Space')
                expect(control).to_have_attribute('aria-checked', initial)
                control.press('Enter')
                expect(control).to_have_attribute('aria-checked', toggled)
                # Restore the selected provider before checking supported tags.
                control.click()
                expect(control).to_have_attribute('aria-checked', initial)

            page.get_by_role('tab', name='Auto tag', exact=True).click()
            exercise(page.get_by_role('checkbox', name='Beatport', exact=True))
            page.get_by_text('Input & Tags', exact=True).click()
            exercise(page.locator('.tags .q-checkbox[data-tag="title"]'))

    def test_autotagger_provider_lookup_order_matches_the_cards_after_reenabling(self):
        with self.session() as (page, backend):
            page.get_by_role("tab", name="Auto tag", exact=True).click()
            beatport = page.get_by_role("checkbox", name="Beatport", exact=True)
            traxsource = page.get_by_role("checkbox", name="Traxsource", exact=True)
            expect(beatport).to_have_attribute("aria-checked", "true")
            traxsource.click()
            beatport.click()
            expect(beatport).to_have_attribute("aria-checked", "false")
            beatport.click()
            expect(beatport).to_have_attribute("aria-checked", "true")
            expect(traxsource).to_have_attribute("aria-checked", "true")
            self.assertEqual(page.locator(".card .q-checkbox").evaluate_all(
                "boxes => boxes.map(box => box.getAttribute('aria-label'))"),
                ["Beatport", "Traxsource"])

            page.get_by_text("Input & Tags", exact=True).click()
            page.get_by_role("button", name="Start tagging", exact=True).click()
            expect(page.get_by_role("button", name="Open results JSON", exact=True)).to_be_visible()
            command = next(c for c in backend.commands if c["action"] == "startTagging")
            self.assertEqual(command["config"]["platforms"], ["beatport", "traxsource"])

    def open_editor(self, page):
        page.get_by_role("tab", name="Edit Tags", exact=True).click()
        expect(page.get_by_text("NO FILE SELECTED", exact=True)).to_be_visible()
        page.locator("#fileList").get_by_text("Synthetic track.mp3", exact=True).click()
        expect(page.get_by_label("Tag TIT2", exact=True)).to_have_value("Synthetic title")

    def test_helper_selection_is_the_tagging_boundary_and_report_is_exposed(self):
        with self.session() as (page, backend):
            page.get_by_role('tab',name='Auto tag',exact=True).click()
            page.get_by_text('Input & Tags',exact=True).click()
            selected={'schema':1,'id':'a'*32,'snapshot':'synthetic','snapshot_digest':'b'*64,
                      'scope_key':'capture:fixture','mapping_revision':'c'*64,
                      'tracks':[{'track_id':'library-1','local_path':'/music/100%20 Café.mp3'}]}
            page.get_by_label('Helper selection manifest (optional)',exact=True).set_input_files({
                'name':'selection.json','mimeType':'application/json','buffer':json.dumps(selected).encode()})
            expect(page.get_by_text('1 selected library track.',exact=False)).to_be_visible()
            expect(page.get_by_label('Path',exact=True)).to_have_count(0)
            page.get_by_role('button',name='Start tagging',exact=True).click()
            expect(page.get_by_role('button',name='Open results JSON',exact=True)).to_be_visible()
            command=next(c for c in backend.commands if c['action']=='startTagging')
            self.assertEqual(command['selection'],selected);self.assertTrue(command['report']);self.assertIsNone(command['playlist'])
            expect(page.get_by_text('Results JSON: /synthetic/results.json',exact=False)).to_be_visible()

    def test_invalid_helper_manifest_blocks_folder_fallback(self):
        with self.session() as (page, _):
            page.get_by_role('tab',name='Auto tag',exact=True).click()
            page.get_by_text('Input & Tags',exact=True).click()
            page.get_by_label('Path',exact=True).fill('/music')
            page.get_by_label('Helper selection manifest (optional)',exact=True).set_input_files({
                'name':'bad.json','mimeType':'application/json','buffer':b'{"schema":2,"tracks":[]}'})
            expect(page.get_by_role('alert')).to_contain_text('Choose a Helper selection manifest')
            expect(page.get_by_role('button',name='Start tagging',exact=True)).to_be_disabled()

    def test_stop_waits_for_completion_retains_results_and_ignores_late_progress(self):
        with self.session() as (page, backend):
            backend.hold_tagging=True
            page.get_by_role('tab',name='Auto tag',exact=True).click()
            page.get_by_text('Input & Tags',exact=True).click()
            page.get_by_label('Path',exact=True).fill('/music')
            page.get_by_role('button',name='Start tagging',exact=True).click()
            stop=page.get_by_role('button',name='Stop tagging',exact=True)
            expect(stop).to_be_visible();stop.click()
            expect(page.get_by_role('status')).to_contain_text('Stopping')
            expect(stop).to_be_disabled()
            backend.tagging_progress()
            expect(page.get_by_text('/music/in-flight.mp3',exact=True)).to_be_visible()
            backend.tagging_socket.send(json.dumps(backend.tagging_done(cancelled=True)))
            expect(stop).to_have_count(0)
            page.get_by_role('button',name='OK',exact=True).click()
            expect(page.get_by_role('status')).to_have_text('Stopped')
            expect(page.locator('.progress')).to_contain_text('25%')
            backend.tagging_progress('/music/late.mp3')
            expect(page.get_by_text('/music/late.mp3',exact=True)).to_have_count(0)
            self.assertEqual(len([c for c in backend.commands if c['action']=='stopTagging']),1)

    def test_failed_stop_acknowledgement_allows_retry(self):
        with self.session() as (page, backend):
            backend.hold_tagging=True;backend.reject_stop=True
            page.get_by_role('tab',name='Auto tag',exact=True).click()
            page.get_by_text('Input & Tags',exact=True).click()
            page.get_by_label('Path',exact=True).fill('/music')
            page.get_by_role('button',name='Start tagging',exact=True).click()
            stop=page.get_by_role('button',name='Stop tagging',exact=True)
            stop.click()
            expect(page.get_by_text('Could not stop tagging',exact=True)).to_be_visible()
            page.get_by_role('button',name='OK',exact=True).click()
            expect(stop).to_be_enabled()
            backend.reject_stop=False;stop.click()
            expect(page.get_by_role('status')).to_contain_text('Stopping')

    def test_welcome_credits_and_session_exchange(self):
        with self.session() as (page, backend):
            expect(page.get_by_text("Developed by Marekkon5 & UI design by Bas Curtiz", exact=False)).to_be_visible()
            expect(page.get_by_text("Build pipelines and security audit by Massimiliano Fantuzzi", exact=False)).to_be_visible()
            self.assertNotIn(TOKEN, page.url)
            self.assertEqual(urlsplit(page.url).fragment, "/")
            self.assertEqual(backend.sessions, [{"method": "POST", "body": json.dumps({"token": TOKEN}, separators=(",", ":"))}])
            self.assertEqual(backend.connections, 1)

    def test_existing_session_uses_get(self):
        with self.session(fragment="#/") as (_, backend):
            self.assertEqual(backend.sessions, [{"method": "GET", "body": None}])

    def test_expired_launch_link_does_not_open_websocket(self):
        with self.session(status=403) as (page, backend):
            expect(page.locator("#app")).to_have_text("This OneTagger session link is invalid or expired.")
            self.assertNotIn(TOKEN, page.url)
            self.assertEqual(backend.connections, 0)
            expect(page.get_by_role("tab")).to_have_count(0)

    def test_missing_session_does_not_open_websocket(self):
        with self.session(status=401, fragment="#/") as (page, backend):
            expect(page.locator("#app")).to_have_text("Open OneTagger using the session link printed by the running application.")
            self.assertEqual(backend.connections, 0)

    def test_navigation_and_file_filter(self):
        with self.session() as (page, _):
            settings_icon = page.get_by_test_id("settings-open").locator(".mdi-cog")
            expect(settings_icon).to_be_visible()
            self.assertTrue(settings_icon.evaluate("""
                async icon => {
                    await document.fonts.ready;
                    const { content, fontFamily } = getComputedStyle(icon, "::before");
                    const iconFont = "Material Design Icons";
                    return content.startsWith('"') && content.length > 2 &&
                        fontFamily.includes(iconFont) &&
                        [...document.fonts].some(font =>
                            font.family.includes(iconFont) && font.status === "loaded"
                        );
                }
            """), "The settings icon should render using the bundled icon font")
            page.get_by_role("tab", name="Edit Tags", exact=True).click()
            expect(page.locator("#fileList .te-file")).to_have_count(2)
            page.get_by_label("Filter", exact=True).fill("synthetic")
            expect(page.locator("#fileList .te-file")).to_have_count(1)
            page.get_by_label("Filter", exact=True).fill("no match")
            expect(page.locator("#fileList .te-file")).to_have_count(0)
            page.get_by_label("Filter", exact=True).fill("")
            expect(page.locator("#fileList .te-file")).to_have_count(2)
            page.get_by_role("tab", name="Home", exact=True).click()
            expect(page.get_by_text("The ultimate cross-platform music tagger for DJs", exact=True)).to_be_visible()

    def test_edit_title_and_save_payload(self):
        with self.session() as (page, backend):
            self.open_editor(page)
            field = page.get_by_label("Tag TIT2", exact=True)
            field.fill("Edited title")
            field.press("Tab")
            page.get_by_role("button", name="Save", exact=True).click()
            expect(page.get_by_text("Tags written!", exact=True)).to_be_visible()
            save = backend.saves()[-1]
            self.assertEqual(save["path"], TRACK["path"])
            self.assertIn({"type": "raw", "tag": "TIT2", "value": ["Edited title"]}, save["changes"])

    def test_remove_art_comment_and_lyrics_updates_correct_items(self):
        with self.session() as (page, backend):
            self.open_editor(page)
            expect(page.get_by_role("button", name="Remove", exact=True)).to_have_count(2)
            page.get_by_role("button", name="Remove", exact=True).first.click()
            expect(page.get_by_role("button", name="Remove", exact=True)).to_have_count(1)
            page.get_by_role("button", name="Remove comment 1", exact=True).click()
            page.get_by_role("button", name="Remove lyrics 1", exact=True).click()
            expect(page.get_by_role("button", name="Remove comment 1", exact=True)).to_have_count(0)
            expect(page.get_by_role("button", name="Remove lyrics 1", exact=True)).to_have_count(0)
            page.get_by_role("button", name="Save", exact=True).click()
            expect(page.get_by_text("Tags written!", exact=True)).to_be_visible()
            changes = backend.saves()[-1]["changes"]
            self.assertIn({"type": "removePicture", "kind": "CoverFront"}, changes)
            self.assertIn({"type": "id3Comments", "comments": []}, changes)
            self.assertIn({"type": "id3UnsynchronizedLyrics", "lyrics": []}, changes)

    def edit_title(self, page, title="Unsaved title"):
        field = page.get_by_label("Tag TIT2", exact=True)
        field.fill(title)
        field.press("Tab")

    def select_other_track(self, page):
        page.locator("#fileList").get_by_text("Another track.flac", exact=True).click()

    def test_failed_save_retains_edits_and_retry_resends_original_changes(self):
        with self.session() as (page, backend):
            self.open_editor(page)
            self.edit_title(page)
            backend.save_failures = 1
            page.get_by_role("button", name="Save", exact=True).click()
            expect(page.get_by_role("alert")).to_contain_text("Your edits are retained")
            expect(page.get_by_text("Tags written!", exact=True)).not_to_be_visible()
            expect(page.get_by_label("Tag TIT2", exact=True)).to_have_value("Unsaved title")
            page.get_by_role("button", name="Save", exact=True).click()
            expect(page.get_by_text("Tags written!", exact=True)).to_be_visible()
            expect(page.get_by_role("alert").filter(has_text="Your edits are retained")).to_have_count(0)
            self.assertEqual(len(backend.saves()), 2)
            self.assertEqual(backend.saves()[0], backend.saves()[1])
            self.assertTrue(backend.saves()[1]["changes"])

    def test_autosave_failure_keeps_current_track_until_retry_succeeds(self):
        with self.session(autosave=True) as (page, backend):
            self.open_editor(page)
            self.edit_title(page)
            backend.save_failures = 1
            self.select_other_track(page)
            expect(page.get_by_role("alert")).to_contain_text("Your edits are retained")
            expect(page.locator(".text-center.monospace")).to_have_text(TRACK["filename"])
            self.assertFalse(any(c["action"] == "tagEditorLoad" and c["path"].endswith(".flac") for c in backend.commands))
            self.select_other_track(page)
            expect(page.locator(".text-center.monospace")).to_have_text("Another track.flac")
            self.assertEqual(backend.saves()[0], backend.saves()[1])

    def test_autosave_waits_for_acknowledgement_before_loading_next_track(self):
        with self.session(autosave=True) as (page, backend):
            self.open_editor(page)
            self.edit_title(page)
            backend.hold_saves = True
            self.select_other_track(page)
            expect(page.get_by_role("status")).to_have_text("Saving changes…")
            expect(page.locator(".text-center.monospace")).to_have_text(TRACK["filename"])
            self.assertEqual(len(backend.pending_saves), 1)
            self.assertFalse(any(c["action"] == "tagEditorLoad" and c["path"].endswith(".flac") for c in backend.commands))
            backend.acknowledge_next_save()
            expect(page.locator(".text-center.monospace")).to_have_text("Another track.flac")
            self.assertEqual(len(backend.saves()), 1)

    def test_autosave_failure_blocks_route_navigation_until_retry(self):
        with self.session(autosave=True) as (page, backend):
            self.open_editor(page)
            self.edit_title(page)
            backend.save_failures = 1
            page.get_by_role("tab", name="Home", exact=True).click()
            expect(page.get_by_role("alert")).to_contain_text("Your edits are retained")
            expect(page.get_by_role("tab", name="Edit Tags", exact=True)).to_have_attribute("aria-selected", "true")
            page.get_by_role("tab", name="Home", exact=True).click()
            expect(page.get_by_text("The ultimate cross-platform music tagger for DJs", exact=True)).to_be_visible()
            self.assertEqual(backend.saves()[0], backend.saves()[1])

    def test_unrelated_save_acknowledgement_cannot_clear_pending_edits(self):
        with self.session() as (page, backend):
            self.open_editor(page)
            self.edit_title(page)
            backend.hold_saves = True
            page.get_by_role("button", name="Save", exact=True).click()
            expect(page.get_by_role("status")).to_have_text("Saving changes…")
            ws, data = backend.pending_saves[0]
            unrelated = backend.save_response(data)
            unrelated["requestId"] = "some-other-request"
            ws.send(json.dumps(unrelated))
            backend.acknowledge_next_save(False)
            expect(page.get_by_role("alert")).to_contain_text("Your edits are retained")
            expect(page.get_by_text("Tags written!", exact=True)).not_to_be_visible()
            backend.hold_saves = False
            page.get_by_role("button", name="Save", exact=True).click()
            expect(page.get_by_text("Tags written!", exact=True)).to_be_visible()
            self.assertEqual(backend.saves()[0], backend.saves()[1])

    def test_disconnect_before_acknowledgement_preserves_editor_values(self):
        with self.session() as (page, backend):
            self.open_editor(page)
            self.edit_title(page)
            backend.hold_saves = True
            page.get_by_role("button", name="Save", exact=True).click()
            expect(page.get_by_role("status")).to_have_text("Saving changes…")
            backend.pending_saves[0][0].close()
            expect(page.get_by_role("alert")).to_contain_text("Your edits are retained")
            expect(page.get_by_label("Tag TIT2", exact=True)).to_have_value("Unsaved title")
            expect(page.get_by_text("Tags written!", exact=True)).not_to_be_visible()
            page.get_by_role("dialog").filter(has_text="WebSocket closed!").get_by_role("button", name="OK", exact=True).click()
            page.get_by_role("button", name="Save", exact=True).click()
            expect(page.get_by_role("alert")).to_contain_text("the connection is closed")
            expect(page.get_by_role("status")).to_have_count(0)
            expect(page.get_by_label("Tag TIT2", exact=True)).to_have_value("Unsaved title")
            self.assertEqual(len(backend.saves()), 1)

    def test_unsaved_track_switch_requires_explicit_discard_without_autosave(self):
        with self.session() as (page, backend):
            self.open_editor(page)
            self.edit_title(page)
            self.select_other_track(page)
            dialog = page.get_by_role("dialog").filter(has_text="Unsaved changes")
            expect(dialog).to_be_visible()
            dialog.get_by_role("button", name="Cancel", exact=True).click()
            expect(page.get_by_label("Tag TIT2", exact=True)).to_have_value("Unsaved title")
            self.select_other_track(page)
            dialog.get_by_role("button", name="Discard changes", exact=True).click()
            expect(page.locator(".text-center.monospace")).to_have_text("Another track.flac")
            self.assertEqual(backend.saves(), [])

    def test_minimum_window_warning_recovers_after_resize(self):
        with self.session() as (page, _):
            page.set_viewport_size({"width": 1100, "height": 700})
            # Quasar may still be animating the separate platform-loading dialog.
            # Identify this warning by its content instead of assuming only one dialog.
            warning = page.get_by_role("dialog").filter(has_text="One Tagger requires")
            expect(warning).to_be_visible()
            expect(warning).to_contain_text("1200x550")
            page.set_viewport_size({"width": 1440, "height": 900})
            expect(warning).to_have_count(0)
            self.assertLessEqual(page.evaluate("document.documentElement.scrollWidth"), 1440)


if __name__ == "__main__":
    failed = False
    summary = []
    for engine in os.environ.get("UI_BROWSERS", "chromium,firefox,webkit").split(","):
        if engine not in {"chromium", "firefox", "webkit"}:
            raise ValueError("Unknown browser engine")
        BrowserTests.engine = engine
        print(f"\nBrowser engine: {engine}", flush=True)
        result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(BrowserTests))
        summary.append({"browser": engine, "tests": result.testsRun, "failures": len(result.failures), "errors": len(result.errors)})
        failed |= not result.wasSuccessful()
    RESULTS.mkdir(exist_ok=True)
    (RESULTS / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as output:
            output.write("### Browser UI regressions\n\n| Engine | Tests | Failures | Errors |\n|---|---:|---:|---:|\n")
            for row in summary:
                output.write(f"| {row['browser']} | {row['tests']} | {row['failures']} | {row['errors']} |\n")
            output.write("\nScreenshots and failure traces: ui-test-results artifact. Synthetic backend; native WebViews and real file writes are not exercised.\n")
    raise SystemExit(1 if failed else 0)
