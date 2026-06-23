# Copperlace Python

Python wrapper for the Copperlace renderer.

The wheel build runs Cargo for `../rust-core`, bundles the resulting native
library, and exposes a small Python API over the Copperlace C ABI.

```python
from copperlace import Copperlace

with Copperlace.from_string('name = ["Mia"]\norigin = "{name}"') as copperlace:
    print(copperlace.render("origin"))
    print(copperlace.render("origin"))
```
