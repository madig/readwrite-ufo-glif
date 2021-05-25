import tempfile
from pathlib import Path

from ufoLib2 import Font

tmp = Path(tempfile.gettempdir())

u = Font.open(tmp / "NotoSans-Bold.ufo")
for g in u:
    pass
u = Font.open(tmp / "NotoSans-CondensedBold.ufo")
for g in u:
    pass
u = Font.open(tmp / "NotoSans-CondensedLight.ufo")
for g in u:
    pass
u = Font.open(tmp / "NotoSans-CondensedSemiBold.ufo")
for g in u:
    pass
u = Font.open(tmp / "NotoSans-Condensed.ufo")
for g in u:
    pass
u = Font.open(tmp / "NotoSans-Light.ufo")
for g in u:
    pass
u = Font.open(tmp / "NotoSans-Regular.ufo")
for g in u:
    pass
u = Font.open(tmp / "NotoSans-SemiBold.ufo")
for g in u:
    pass
