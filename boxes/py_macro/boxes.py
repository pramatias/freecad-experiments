#exec(open("/home/emporas/repos/freecad/rust/boxes/boxes-py/py_macro/boxes.py").read())

import os
import sys
import time
import uuid
import shutil
import pathlib
import importlib.util

_SP = pathlib.Path("/home/emporas/.venv/lib/python3.12/site-packages")

def _load_fresh():
    # adjust this directory name to match the installed package folder
    pkg_dir = _SP / "boxes_py"
    src = next(pkg_dir.glob("boxes_py.cpython-*.so"))

    tmp_name = f"_sm_{time.time_ns()}_{os.getpid()}.so"
    tmp = pkg_dir / tmp_name
    shutil.copy2(src, tmp)

    module_name = f"boxes_sm_{uuid.uuid4().hex}"
    sys.modules.pop(module_name, None)

    spec = importlib.util.spec_from_file_location(module_name, tmp)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod

sm = _load_fresh()

def build_boxes(*args, **kwargs):
    return sm.build_boxes(*args, **kwargs)

build_boxes()
