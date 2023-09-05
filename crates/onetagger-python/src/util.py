import io
import sys
import onetagger

# stdout -> log
class OneTaggerInfoLog(io.TextIOBase):
    def __init__(self):
        pass
    def write(self, v):
        if v.strip() != '':
            onetagger.info(v)
sys.stdout = OneTaggerInfoLog()

# stderr -> log
class OneTaggerErrorLog(io.TextIOBase):
    def __init__(self):
        pass
    def write(self, v):
        if v.strip() != '':
            onetagger.error(v)
sys.stderr = OneTaggerInfoLog()
