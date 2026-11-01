"""Guards for the loading table of the Data page: the last column holds the download icon + move + remove without clipping (it also stays in view when the table scrolls sideways), the headers have room, a file name is not underlined, and a maker's sample shows a real size."""
import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"


def test_last_column_fits_its_buttons():
    css = (WEB / "app.css").read_text(encoding="utf-8")
    act = int(re.search(r"\.ldtab col\.c-act\{width:(\d+)px\}", css).group(1))
    assert act >= 112, "download icon + move + × need about 108 px"
    assert "position:sticky;right:0" in css, "the actions stay in view when the table scrolls sideways"
    tim = int(re.search(r"\.ldtab col\.c-tim\{width:(\d+)px\}", css).group(1)); siz = int(re.search(r"\.ldtab col\.c-siz\{width:(\d+)px\}", css).group(1))
    assert tim >= 170 and siz >= 108, "the Time header with its unit select and «Dimensione» are not clipped"
    lnk = re.search(r"\.ldtab button\.lnk\{[^}]*\}", css).group(0)
    assert "text-decoration:none" in lnk, "file names are not underlined (underscores stay readable)"
    fixed = sum(int(m) for m in re.findall(r"\.ldtab col\.c-(?:use|tim|con|siz|act)\{width:(\d+)px\}", css))
    pct = sum(int(m) for m in re.findall(r"\.ldtab col\.c-(?:exp|typ)\{width:(\d+)%\}", css))
    minw = int(re.search(r"\.ldtab\{[^}]*min-width:(\d+)px", css).group(1))
    # the file column (no width) keeps at least 160 px at the minimum width
    assert minw * (100 - pct) / 100 - fixed >= 160, (minw, fixed, pct)


def test_native_rows_show_the_sum_of_the_members_when_the_sample_size_is_unknown():
    src = (WEB / "rust-worker.js").read_text(encoding="utf-8")
    assert "const shownSize = " in src and "st.dim != null ? st.dim : st.size" in src
    assert src.count("shownSize(st)") == 2, "lr_list and lr_entry"
    assert "st.dim != null ? st.dim : null" not in src
