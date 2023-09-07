import pdoc
import requests
from bs4 import BeautifulSoup

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
    
    with open(output, 'w') as f:
        f.write(soup.prettify())