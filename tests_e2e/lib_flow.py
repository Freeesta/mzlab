"""Helpers of the e2e tests that drive a campaign folder through Flow (the «Confidence level» node): open_campaign(pg, folder) opens Flow, adds the node and gives it the folder."""
import json
from pathlib import Path
from lib import attendi

RESULTS = ".flx-res"       # the results block under the canvas: the table, Review, the exports and the MS2 panel of the campaign
FOLDER_INPUT = ".fl-insp input[type=file][webkitdirectory]"


def open_campaign(pg, folder, timeout=180000):
    """Flow tab -> node «Confidence level» (added once per page) -> the folder; returns when the table has rows."""
    if not pg.locator(FOLDER_INPUT).count():
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_selector("#flx-add"); pg.click("#flx-add"); pg.click('.flx-pop button:has-text("Confidence level")')
        pg.click('.fln:has-text("Confidence level")')
        attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Confidence level'")
    pg.set_input_files(FOLDER_INPUT, str(folder))
    name = json.dumps(Path(folder).name + " (campaign folder)")
    attendi(pg, f"[...document.querySelectorAll('h3')].some(h => h.textContent.includes({name})) && document.querySelectorAll('.flx-res .tp tbody tr.clk').length > 0", timeout)
