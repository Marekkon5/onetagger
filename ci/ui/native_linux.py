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
        interactions = []
        layouts = []
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
                        # A routed page can exist before its fade/step transition
                        # finishes. Do not measure coordinates during that interval.
                        wait("""const e=document.querySelector(arguments[0]);
                            return !!e && !document.querySelector('.fade-enter-active,.fade-leave-active') &&
                                document.getAnimations().every(a =>
                                !a.effect?.target?.contains(e) ||
                                !Number.isFinite(a.effect.getComputedTiming().endTime) ||
                                (a.playState !== 'running' && !a.pending));""",selector)
                        position_code = """const e=document.querySelector(arguments[0]);
                            if(!e)return null;const r=e.getBoundingClientRect();
                            const x=Math.round(r.left+r.width/2),y=Math.round(r.top+r.height/2);
                            const hit=document.elementFromPoint(x,y);
                            return r.width && r.height && hit && e.contains(hit) ? {x,y} : null;"""
                        position = script(position_code, selector)
                        # Scroll only the app's content panel. scrollIntoView can
                        # move multiple ancestors during a route transition and
                        # considers controls underneath the fixed header visible.
                        if not position:
                            script("""const e=document.querySelector(arguments[0]);
                                const panel=e?.closest('.content');
                                if(panel){
                                    const r=e.getBoundingClientRect(),p=panel.getBoundingClientRect();
                                    const header=document.querySelector('.q-header')?.getBoundingClientRect();
                                    const top=Math.max(p.top,header?.bottom || 0)+8;
                                    const bottom=Math.min(p.bottom,innerHeight)-80;
                                    panel.scrollTop += (r.top+r.height/2)-(top+bottom)/2;
                                }""",selector)
                            time.sleep(.2)
                            position = script(position_code, selector)
                        time.sleep(.2)
                        settled = script(position_code, selector)
                        if not position or settled != position:
                            raise RuntimeError('element not interactable: layout is moving or covered')
                        # Moving the native pointer can trigger hover/layout work
                        # after the DOM scroll has settled. Complete that action
                        # separately and recheck the hit target before pressing.
                        command('/actions', {'actions':[{'type':'pointer','id':'mouse',
                            'parameters':{'pointerType':'mouse'},'actions':[
                                {'type':'pointerMove','duration':100,'origin':'viewport',**position},
                                {'type':'pause','duration':100}]}]})
                        if script(position_code, selector) != position:
                            raise RuntimeError('element not interactable: target moved after pointer motion')
                        script("window.nativeClickTarget=document.querySelector(arguments[0]); window.nativePointerEvents=[];",selector)
                        command('/actions', {'actions':[{'type':'pointer','id':'mouse',
                            'parameters':{'pointerType':'mouse'},'actions':[
                                {'type':'pointerDown','button':0},
                                {'type':'pause','duration':100},
                                {'type':'pointerUp','button':0}]}]})
                        events = script('return window.nativePointerEvents')
                        interactions.append({'selector':selector,'position':position,'events':events})
                        if not any(e['type']=='click' and e['intended'] and e['trusted'] for e in events):
                            raise AssertionError('Native click missed its target: '+json.dumps(interactions[-1]))
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
                script("""window.nativeErrors=[]; window.nativePointerEvents=[];
                    addEventListener('error',e=>nativeErrors.push(e.message));
                    addEventListener('unhandledrejection',e=>nativeErrors.push(String(e.reason)));
                    for (const type of ['pointerdown','pointerup','click']) {
                        document.addEventListener(type,e=>{
                            const control=e.target.closest('.q-checkbox,a,.q-stepper__tab');
                            nativePointerEvents.push({type, trusted:e.isTrusted,
                                intended:!!window.nativeClickTarget?.contains(e.target),
                                label:control?.getAttribute('aria-label') || control?.innerText,
                                tag:control?.dataset.tag, x:e.clientX, y:e.clientY,
                                scrollTop:document.querySelector('.content')?.scrollTop,
                                windowScrollY:window.scrollY,
                                targetRect:window.nativeClickTarget?.getBoundingClientRect().toJSON(),
                                active:document.activeElement?.outerHTML.slice(0,300)});
                        },true);
                    }""")
                for cycle, (width, height) in enumerate([(1280, 720), (1440, 900), (1200, 600)]):
                    command('/window/rect', {'width': width, 'height': height})
                    click('a[href="#/autotagger"]')
                    # Keep the same provider identity across async renders and
                    # route changes instead of following whichever card is first.
                    selector = '.card .q-checkbox[aria-label="Beatport"]'
                    wait("return !!document.querySelector(arguments[0])",selector)
                    for toggle in range(10):
                        before = script("return document.querySelector(arguments[0]).getAttribute('aria-checked')",selector)
                        click(selector+' .q-checkbox__inner')
                        wait("return document.querySelector(arguments[0]).getAttribute('aria-checked') !== arguments[1]",selector,before)
                        print(f'Native cycle {cycle+1}: provider toggle {toggle+1}/10 passed',flush=True)
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
                    # Scroll a long panel, then ensure a newly selected panel
                    # starts below the fixed app header with no retained offset.
                    script("const p=document.querySelector('.content');p.scrollTop=p.scrollHeight")
                    wait("return document.querySelector('.content').scrollTop > 100")
                    click('.q-stepper__tab:nth-child(1)')
                    wait("return document.querySelector('.content').scrollTop === 0")
                    geometry = script("""const p=document.querySelector('.content');
                        const inner=document.querySelector('.q-stepper__step-inner');
                        const step=inner.closest('.q-stepper__step');
                        return {unused:step.getBoundingClientRect().height-inner.getBoundingClientRect().height,
                            scrollTop:p.scrollTop, viewport:[innerWidth,innerHeight],
                            documentScroll:document.documentElement.scrollHeight-innerHeight,
                            stepHeight:step.getBoundingClientRect().height,
                            contentHeight:p.scrollHeight};""")
                    layouts.append(geometry)
                    if geometry['unused'] > 2 or geometry['documentScroll'] > 2:
                        raise AssertionError('Native layout contains unused space: '+repr(geometry))
                    click('a[href="#/"]')
                    wait("return document.body.innerText.includes('The ultimate cross-platform music tagger for DJs')")
                errors = script('return window.nativeErrors')
                if errors: raise AssertionError('Native JavaScript errors: '+repr(errors))
                args.output.joinpath('result.json').write_text(json.dumps({
                    'status':'passed','cycles':3,'provider_checkbox_clicks':30,'tag_checkbox_clicks':18,
                    'engine':'system WebKitGTK via real OneTagger executable','provider_requests':0},indent=2))
                print('Native Linux Autotagger navigation and repeated controls passed')
            finally:
                args.output.joinpath('layouts.json').write_text(json.dumps(layouts,indent=2))
                # Collect bounded CI-only diagnostics even when JS is unresponsive.
                for name, cmd in [
                    ('system.txt', ['uname', '-a']),
                    ('webkit-packages.txt', ['dpkg-query', '-W', 'libwebkit2gtk-4.1-0', 'libgtk-3-0t64', 'libgl1-mesa-dri']),
                    ('processes.txt', ['ps', '-eo', 'pid,ppid,stat,pcpu,pmem,rss,comm']),
                ]:
                    try:
                        result = subprocess.run(cmd, capture_output=True, text=True, timeout=5)
                        args.output.joinpath(name).write_text(result.stdout+result.stderr)
                    except (OSError, subprocess.TimeoutExpired):
                        pass
                args.output.joinpath('interactions.json').write_text(json.dumps(interactions,indent=2))
                if session:
                    try:
                        diagnostics = script("return [...document.querySelectorAll('.q-checkbox,.q-stepper__tab')].map(e=>({text:e.innerText,label:e.getAttribute('aria-label'),tag:e.dataset.tag,checked:e.getAttribute('aria-checked'),disabled:e.getAttribute('aria-disabled'),rect:e.getBoundingClientRect().toJSON()}))")
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
