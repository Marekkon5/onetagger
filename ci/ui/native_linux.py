"""Exercise the real Linux Wry/WebKitGTK app and backend on GitHub, without providers."""
import argparse
import base64
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import tempfile
import time
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('binary', type=Path)
    parser.add_argument('--output', type=Path, default=Path('native-ui-results'))
    args = parser.parse_args()
    args.output.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='onetagger-native-') as tmp:
        root = Path(tmp)
        # Only the child application's directories are changed; no user settings.
        env = dict(os.environ, XDG_DATA_HOME=str(root/'data'), XDG_CONFIG_HOME=str(root/'config'),
                   XDG_CACHE_HOME=str(root/'cache'))
        music = root/'Empty music Café'; music.mkdir()
        with socket.socket() as sock:
            sock.bind(('127.0.0.1', 0)); port = sock.getsockname()[1]
        log = root/'driver.log'
        session = None
        with log.open('w') as output:
            driver = subprocess.Popen(['WebKitWebDriver', '--port='+str(port)], env=env,
                                      stdout=output, stderr=subprocess.STDOUT)
            base = 'http://127.0.0.1:'+str(port)
            def call(method, path, body=None):
                data = None if body is None else json.dumps(body).encode()
                try:
                    with urlopen(Request(base+path, data=data, method=method,
                                         headers={'Content-Type':'application/json'}), timeout=30) as response:
                        result = json.load(response)
                except HTTPError as error:
                    raise RuntimeError(error.read().decode()) from error
                return result.get('value')
            def command(path, body=None):
                return call('POST' if body is not None else 'GET', '/session/'+session+path, body)
            def script(code, *values):
                return command('/execute/sync', {'script':code, 'args':list(values)})
            def wait(code, *values):
                deadline = time.monotonic()+30
                while time.monotonic()<deadline:
                    if script(code, *values): return
                    time.sleep(.2)
                raise AssertionError('Native UI did not reach expected state: '+code)
            def click(selector):
                deadline = time.monotonic()+15
                while True:
                    try:
                        # WebKit's automatic top alignment can put the target
                        # underneath the fixed navigation bar. Center it first;
                        # the action itself still uses native pointer input.
                        script("document.querySelector(arguments[0])?.scrollIntoView({block:'center',inline:'center',behavior:'instant'});", selector)
                        position_code = """const e=document.querySelector(arguments[0]);
                            if(!e)return null;const r=e.getBoundingClientRect();
                            const x=Math.round(r.left+r.width/2),y=Math.round(r.top+r.height/2);
                            const hit=document.elementFromPoint(x,y);
                            return r.width && r.height && hit && e.contains(hit) ? {x,y} : null;"""
                        position = script(position_code, selector)
                        time.sleep(.2)
                        settled = script(position_code, selector)
                        if not position or settled != position:
                            raise RuntimeError('element not interactable: layout is moving or covered')
                        # WebKit's Element Click may scroll a nested Quasar panel
                        # again after locating the element. Pointer Actions uses
                        # the settled, hit-tested position without that extra scroll.
                        command('/actions', {'actions':[{'type':'pointer','id':'mouse',
                            'parameters':{'pointerType':'mouse'},'actions':[
                                {'type':'pointerMove','duration':0,'origin':'viewport',**position},
                                {'type':'pointerDown','button':0},{'type':'pointerUp','button':0}]}]})
                        return
                    except RuntimeError as error:
                        if time.monotonic()>=deadline or not any(reason in str(error) for reason in
                                ('element click intercepted','element not interactable','stale element reference')):
                            raise
                        # Wait for actual animated page transitions, never replace
                        # native input with a JavaScript click that bypasses hit tests.
                        time.sleep(.2)
            try:
                for _ in range(100):
                    if driver.poll() is not None: raise RuntimeError('WebKitWebDriver exited')
                    try: call('GET','/status'); break
                    except URLError: time.sleep(.1)
                value = call('POST','/session', {'capabilities':{'alwaysMatch':{
                    'browserName':'wry', 'webkitgtk:browserOptions':{
                        'binary':str(args.binary.resolve()), 'args':['--webdriver']}}}})
                session = value['sessionId']
                handles = command('/window/handles')
                if not handles: raise AssertionError('Native application has no window')
                command('/window', {'handle':handles[0]})
                wait("return !!document.querySelector('a[href=\"#/autotagger\"]') && !document.body.innerText.includes('Loading platforms...')")
                script("window.nativeErrors=[]; addEventListener('error',e=>nativeErrors.push(e.message)); addEventListener('unhandledrejection',e=>nativeErrors.push(String(e.reason)))")
                for cycle in range(3):
                    click('a[href="#/autotagger"]')
                    wait("return document.querySelectorAll('.card .q-checkbox').length > 0")
                    for _ in range(10):
                        before = script("return document.querySelector('.card .q-checkbox').getAttribute('aria-checked')")
                        click('.card .q-checkbox')
                        wait("return document.querySelector('.card .q-checkbox').getAttribute('aria-checked') !== arguments[0]",before)
                    click('.q-stepper__tab:nth-child(2)')
                    wait("return !!document.querySelector('input[aria-label=\"Path\"]')")
                    script("const e=document.querySelector('input[aria-label=\"Path\"]'); e.value=arguments[0]; e.dispatchEvent(new Event('input',{bubbles:true}));",str(music))
                    for _ in range(6):
                        selector = '.tags .q-checkbox[data-tag="title"]'
                        before = script("return document.querySelector(arguments[0]).getAttribute('aria-checked')",selector)
                        click(selector+' .q-checkbox__inner')
                        wait("return document.querySelector(arguments[0]).getAttribute('aria-checked') !== arguments[1]",selector,before)
                    click('.q-stepper__tab:nth-child(4)')
                    wait("return document.body.innerText.includes('Miscellaneous options')")
                    click('a[href="#/"]')
                    wait("return document.body.innerText.includes('The ultimate cross-platform music tagger for DJs')")
                errors = script('return window.nativeErrors')
                if errors: raise AssertionError('Native JavaScript errors: '+repr(errors))
                args.output.joinpath('result.json').write_text(json.dumps({
                    'status':'passed','cycles':3,'provider_checkbox_clicks':30,'tag_checkbox_clicks':18,
                    'engine':'system WebKitGTK via real OneTagger executable','provider_requests':0},indent=2))
                print('Native Linux Autotagger navigation and repeated controls passed')
            finally:
                if session:
                    try:
                        diagnostics = script("return [...document.querySelectorAll('.q-checkbox,.q-stepper__tab')].map(e=>({text:e.innerText,tag:e.dataset.tag,checked:e.getAttribute('aria-checked'),disabled:e.getAttribute('aria-disabled'),rect:e.getBoundingClientRect().toJSON()}))")
                        args.output.joinpath('controls.json').write_text(json.dumps(diagnostics,indent=2))
                    except Exception: pass
                    try: args.output.joinpath('native.png').write_bytes(base64.b64decode(command('/screenshot')))
                    except Exception: pass
                    try: call('DELETE','/session/'+session)
                    except Exception: pass
                driver.terminate()
                try: driver.wait(timeout=10)
                except subprocess.TimeoutExpired: driver.kill(); driver.wait()
                output.flush()
                args.output.joinpath('driver.log').write_text(re.sub(r'#session=[0-9a-f]+','#session=REDACTED',log.read_text(errors='replace')))


if __name__ == '__main__':
    main()
