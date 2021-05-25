import tempfile
from pathlib import Path

from ufoLib2 import Font

tmp = Path(tempfile.gettempdir())

Font.open(tmp / "NotoSans-Bold.ufo", lazy=False)
Font.open(tmp / "NotoSans-CondensedBold.ufo", lazy=False)
Font.open(tmp / "NotoSans-CondensedLight.ufo", lazy=False)
Font.open(tmp / "NotoSans-CondensedSemiBold.ufo", lazy=False)
Font.open(tmp / "NotoSans-Condensed.ufo", lazy=False)
Font.open(tmp / "NotoSans-Light.ufo", lazy=False)
Font.open(tmp / "NotoSans-Regular.ufo", lazy=False)
Font.open(tmp / "NotoSans-SemiBold.ufo", lazy=False)
