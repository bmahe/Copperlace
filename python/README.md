# Copperlace Python

Python wrapper for the Copperlace renderer.

The wheel build runs Cargo for `../rust-core`, bundles the resulting native
library, and exposes a small Python API over the Copperlace C ABI.

```python
from copperlace import Copperlace

with Copperlace.from_string('name = ["Mia"]\norigin = "{name}"') as copperlace:
    print(copperlace.render("origin"))
    print(copperlace.render("origin"))
    print(copperlace.render("origin", {"name": "Darcy"}))

with Copperlace.from_string(
    'name = ["Mia"]\norigin = "{name | shout}"',
    {"shout": lambda value: value.upper()},
) as copperlace:
    print(copperlace.render("origin"))
```

Structured rendering returns native Python values for object-valued rules:

```python
from copperlace import render_str_structured

config = """
name = ["Mia"]
origin {
  title = "Hello {name}"
  tags = ["structured", "{name | slug}"]
}
"""

print(render_str_structured(config, "origin"))
# {"tags": ["structured", "mia"], "title": "Hello Mia"}
```

`RuleSet.render_structured`, `Copperlace.render_structured`,
`render_str_structured`, and `render_file_structured` use compact native JSON
internally and parse it into `dict`, `list`, scalar, boolean, and `None` values
before returning.

Use inferred rendering when callers want the CLI-style behavior from one method:
text rules return text, list rules keep random text choice behavior, and
object-valued rules return a formatted JSON string.

```python
from copperlace import render_str_inferred

print(render_str_inferred(config, "origin"))
```
