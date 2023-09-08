import pdoc
import requests
from bs4 import BeautifulSoup

STYLE = """
body {
    background-color: #181818;
    color: #fffff8;
}

a {
    color: #00d2bf;
}

pre {
    background-color: #202020 !important;
    color: #fffff8 !important;
}

code {
    background-color: #202020 !important;
    color: #fffff8 !important;
}

.hljs-keyword {
    color: #c791e8;
}

.hljs-title {
    color: #00d2bf;
}

.hljs-string {
    color: #c2e58a;
}

.hljs-literal {
    color: #ef632a;
}

.ident {
    color: #00d2bf;
}
"""

def generate_docs(module, output):
    html = pdoc.html(module)
    soup = BeautifulSoup(html, features='lxml')

    # Make CSS offline
    for e in soup.select('link'):
        if e.has_attr('href'):
            r = requests.get(e.attrs['href'])
            new = soup.new_tag('style')
            new.string = r.text
            e.replace_with(new)
        
    # Make JS offline
    for e in soup.select('script'):
        if e.has_attr('src'):
            r = requests.get(e.attrs['src'])
            new = soup.new_tag('script')
            new.string = r.text
            e.replace_with(new)

    # Cleanup
    for e in soup.select('div.desc'):
        if 'Return an attribute of instance, which is of type owner.' in e.text.strip():
            e.replace_with('')

    # Inject style
    style = soup.new_tag('style')
    style.string = STYLE
    soup.body.append(style)

    with open(output, 'w') as f:
        f.write(soup.prettify())