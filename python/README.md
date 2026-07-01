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

Recursive rule references are errors by default. Pass `max_recursion_depth` to
allow limited recursive expansion; recursive calls beyond the limit return an
empty string:

```python
from copperlace import render_str

print(render_str('origin = "x{origin}"', "origin", max_recursion_depth=1))
# xx
```

Structured rendering returns JSON strings for object-valued rules:

```python
from copperlace import render_str_structured

config = """
name = ["Mia"]
origin {
  title = "Hello {name}"
  tags = ["structured", "{name | slug}"]
  count = 3
  active = true
  missing = null
}
"""

print(render_str_structured(config, "origin"))
# {
#     "active": true,
#     "count": 3,
#     "missing": null,
#     "tags": [
#         "structured",
#         "mia"
#     ],
#     "title": "Hello Mia"
# }
```

`RuleSet.render_structured`, `Copperlace.render_structured`,
`render_str_structured`, and `render_file_structured` return formatted JSON
strings from the native renderer.

Use inferred rendering when callers want the CLI-style behavior from one method:
text rules return text, list rules keep random text choice behavior, and
object-valued rules return a formatted JSON string.

```python
from copperlace import render_str_inferred

print(render_str_inferred(config, "origin"))
```
