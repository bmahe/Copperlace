# Copperlace Python

Python wrapper for the Copperlace renderer.

The wheel build runs Cargo for `../rust-core`, bundles the resulting native
library, and exposes a small Python API over the Copperlace C ABI.

```python
from copperlace import RuleSet

rules = RuleSet.from_string('name = ["Mia"]\norigin = "{name}"')
print(rules.render("origin"))
```
