# Campagna sintetica di Attributio

Campagna INVENTATA (nessun dato vero): uguale alla fixture `tests/fixtures/synthetic_campaign/` di Attributio 1.12.0 più `livelli_attesi.csv` (15 feature: POS 1-6 e 8-14, NEG 1-2). Serve ai dati d'oro di `tests/golden/attributio/sintetica/` e alle prove di parità del passaggio a Rust (`docs/agenti/attributio.md`).

| File (in `campagna_sintetica/`) | sha256 |
|---|---|
| `02 Data/Data Synth/1 - mzMine/DDA_NEG/merged/merged_neg.csv` | `ded8120213cf66fe54165a682576f92304ea03120fc130ef7ad537ac0caac0a6` |
| `02 Data/Data Synth/1 - mzMine/DDA_POS/merged/merged_pos.csv` | `904f369e08b943624b5c5cb6e5ac902a992d7653485ca58ef705fa0bd6ad8346` |
| `02 Data/Data Synth/1 - mzMine/DDA_POS/merged/merged_pos_annotations.csv` | `d982bd4dad9d86569e5bcfe2ff0389d980966b0450bd65c04fa0ff7e55684baa` |
| `02 Data/Data Synth/1 - mzMine/DDA_POS/merged/merged_pos_gnps.mgf` | `6e27b3656d2dbcbdbe4af0fedf88015308036eec9c2883d5010f34eecaaf9e28` |
| `02 Data/Data Synth/2 - SIRIUS/output NEG/canopus_structure_summary.tsv` | `5e4939b8e2cc47b5fed18158e72f7fbbc1bcaa43fbeefed6ff2edbfbb2ab447d` |
| `02 Data/Data Synth/2 - SIRIUS/output POS/canopus_structure_summary.tsv` | `7affa1dbb3df45035d80b4c4510646460fb1eab23cb977739b2db805b9771216` |
| `02 Data/Data Synth/2 - SIRIUS/output POS/spectral_matches.tsv` | `ae10178bf74d080b67b809130d573326d7d4d267ef0017f67c39ac0ca5d29bef` |
| `02 Data/Data Synth/2 - SIRIUS/output POS/structure_identifications.tsv` | `df7dcc6e6f98759416dd5163112603c67ce8877aa315cf8d31251cc113a23a37` |
| `02 Data/Data Synth/3 - GNPS/GNPS2_output_pos/nf_output/library/merged_results_with_gnps.tsv` | `d45047b5668b10f252b706369067bae9bdd5f23de403b0113f5cc8160a6dd7b1` |
| `02 Data/Data Synth/3 - GNPS/GNPS2_output_pos/nf_output/networking/clustersummary_with_network.tsv` | `27c584990e38f4e8fb276e1ba427eae81b421dc0328ece329b7bc13a43299af1` |
| `03 Stats/scripts/qc_expected.toml` | `1d0dd7a1e5dfed0423a077ed19f6dd3568e83d68cef79ef867d39b539398ab62` |
| `03 Stats/scripts/shared/feature_decisions.csv` | `62282334517709db9b4911b045d618cb5f03adefce37e96b89b9d17a161835dc` |
| `campaign.toml` | `4b24076dcae543b9c2c2658772d6339adb423b8a11122a4f437eb04580313032` |
| `library/ms2_pos.json` | `05d845806d09b3c77fe546cd7eca75e579b40f33a21879e272c870bad3f01830` |
| `library/standards.csv` | `89072ddab2272840b11d7511bbc9c9f3b014dd3119f494352a35084c1bc3b2b2` |
| `livelli_attesi.csv` | `6712b2f1e5ca1ddb489a1c1948fff85a5ea8b439fb0aa6eb439c94e375f4887c` |

## Campagna dei sorbenti

Campagna INVENTATA (nessun dato vero) con due sorbenti (OA, EC), una lista per sorbente e due feature gemelle (id 3 → 2); serve ai dati d'oro di `tests/golden/attributio/sorbenti/`.

| File (in `campagna_sorbenti/`) | sha256 |
|---|---|
| `campaign.toml` | `228a7a30bccfcef6f1c58be14406df6105c3068850b86fe56152105384f9ecab` |
| `data/mz/DDA_POS/EC/list_EC.csv` | `f8ee12386e98e3fa92087aace40c387060eb13b71c77db9e20da8489027b69c3` |
| `data/mz/DDA_POS/OA/list_OA.csv` | `4369a8e1642c6ff935a5b80b92157745b18e88a5fb7c57b405a8e592e1d884f9` |
| `data/mz/DDA_POS/merged/merged_pos.csv` | `f77cd19ab5a5c71c46fc5dcc078118367d234aebef972928d48714c21215d9ca` |

Inoltre `librerie_sintetiche/` (MGF, MSP, JSON a righe, MassBank) e `fonti_sintetiche/` (tabelle di SIRIUS e GNPS2 per POS e NEG): dati inventati per lo stadio `abbinamenti`.

Inoltre `campagna_banco/` (sovrapposizione sulla campagna sintetica per il Banco 2b: aggiunge MS2, tabelle SIRIUS/GNPS2 e una libreria inventate; dati inventati):

| File (in `campagna_banco/`) | sha256 |
|---|---|
| `02 Data/Data Synth/1 - mzMine/DDA_POS/merged/merged_pos.csv` | `d6ebff7582a3d8e2d91eaf450a6144b3f6d996917326c3a8ddea568a73c70346` |
| `02 Data/Data Synth/1 - mzMine/DDA_POS/merged/merged_pos_gnps.mgf` | `4b5fc78595b3820fa004bc502998517ac98920a7f049451b326f1d69f41411b5` |
| `02 Data/Data Synth/2 - SIRIUS/output POS/formula_identifications_top-5.tsv` | `c41b3758d20500f228c218989daa4ca03f69786711a258e60257afe1ce0c7a6c` |
| `02 Data/Data Synth/2 - SIRIUS/output POS/structure_identifications.tsv` | `30810ee505c65ed567375fa03e5c1b1f5a436fb1cb5c9a1fa1b423542347c21e` |
| `02 Data/Data Synth/2 - SIRIUS/output POS/structure_identifications_top-5.tsv` | `417e455997f7168c7c83d06932e1841e997e54cdef36a1f159a5a9f7292b6d37` |
| `library/banco.json` | `b1e2329a84db082e0b7df377e70ea15eeb320dbeb9cf84d22e757effcbd5dda2` |

Inoltre `campagna_esportazione/` (sovrapposizione sulla campagna sintetica per le esportazioni: decisioni 2b e curva di taratura inventate) e `campagna_gradiente/` (due siti a monte e a valle, per il gradiente); dati inventati:

| File | sha256 |
|---|---|
| `campagna_esportazione/03 Stats/scripts/shared/feature_decisions.csv` | `5cb6d0d2d5c0fef1bd563ffb76f71bcbf8db7c312c1e4a8391c926c9631d6394` |
| `campagna_esportazione/campaign.toml` | `2fad6c66202b493ef243cc503c8334c32f823be5b28cbdbe051b792cf49100a3` |
| `campagna_esportazione/library/calibration.csv` | `8dbc493510571cb597caf57dfcbe99df636e18b71f5bdb2fd3599b62f13671bc` |
| `campagna_gradiente/campaign.toml` | `0ef2c287ffc03abb7f7c9fa47c4c9fabef48f3d6427645f67d3487a2803f0239` |
| `campagna_gradiente/data/mz/DDA_NEG/merged/merged_neg.csv` | `0e4e6aeef1b3407fa5e2f41a3d938c481bbb5629cec5323949b55983f2abf012` |
| `campagna_gradiente/data/mz/DDA_POS/merged/merged_pos.csv` | `4a9acb9cf199e943c7c9f4e7e7d39bb35e8051b1a9d4b6a2860fcb493f53f9a5` |
