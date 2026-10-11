---
title: 'mzLab: a browser-based teaching tool for LC-MS data analysis and the search for transformation products'
tags:
  - mass spectrometry
  - LC-MS
  - transformation products
  - non-target screening
  - chemical education
  - analytical chemistry
  - WebAssembly
authors:
  - name: Federico Cristaudo
    orcid: 0000-0000-0000-0000 # TODO: to be filled in by the author
    affiliation: 1
affiliations:
  - name: Università degli Studi di Torino, Italy
    index: 1
date: 10 October 2026
bibliography: paper.bib
---

# Summary

mzLab is a free teaching program that lets students analyse real liquid chromatography–mass spectrometry (LC-MS) data directly in a web browser. It was written for an undergraduate laboratory experience in which students search for the transformation products (TPs) of a pollutant degraded by TiO~2~ photocatalysis, using data acquired on a triple-quadrupole instrument at unit resolution. Students choose what to extract, integrate, annotate and draw; the program displays the data (total and extracted ion chromatograms, spectra, retention-time × m/z maps, peak integration, adducts, neutral losses, isotope patterns) but does not tell them which signals are TPs. A structure editor and a set of theory chapters and practice exercises (in Italian) accompany the data view. For research-oriented use, mzLab also reads high-resolution (Orbitrap, Q-TOF) and data-dependent-acquisition files and offers formula, isotope and spectral-library search tools whose output is presented as scored candidates with supporting evidence, never as established identifications. Everything runs client-side (Python and numpy compiled to WebAssembly through Pyodide, plus JavaScript); there is nothing to install, no account, and no tracking.

# Statement of need

Non-target and suspect screening of environmental contaminants and their TPs is now a routine part of environmental analytical chemistry, with community guidance on quality assurance, data processing and reporting of identification confidence [@hollender2023; @schymanski2014]. It is also a delicate skill: different software applied to the same raw files returns largely different feature lists [@hohrenk2020], TP formation is difficult to predict even for well-studied oxidation processes [@minakata2023; @loffler2025], and quantification of TPs without reference standards can be off by large factors [@malm2024]. These are exactly the points an analytical chemist-in-training should meet and reason about.

Teaching resources do not follow this lead. We did not find, in the literature we searched (including the *Journal of Chemical Education*, 2020–2026), a report on teaching non-target screening or TP identification; the closest are undergraduate laboratory experiences in LC-HRMS metabolomics [@boyce2019]. This is a negative search result, not proof of absence. In practice, the tools available to instructors fall into two groups. Educational mass-spectrometry applications generally use simulated or static spectra, and so cannot reproduce the experience of reading a real chromatogram with noise, co-eluting peaks and unit-resolution ambiguity. Research tools such as MZmine [@schmid2023] and similar platforms work on real data but require installation, user accounts, licences or uploading files to a server, are designed for high-resolution data, and expose a large parameter space aimed at experts. Triple-quadrupole data at unit resolution, still the workhorse of many teaching laboratories, are largely ignored by metabolomics software.

mzLab fills this gap: real instrument files, analysed in the browser, with no installation, no account and no upload, in a form suited to a single laboratory session.

# Functions and instructional design

The central design rule is that **the program does not give the answers**. In the low-resolution (teaching) mode, mzLab never states "these are the TPs", never produces calibration lines, and never produces the tables of the lab report: students compute these themselves in a spreadsheet. The program lowers the cost of looking at the data (extracting ions, integrating peaks, comparing spectra) so that class time goes to interpretation. A m/z at unit resolution is treated as a *candidate*, not an identification, a point developed in the theory chapters rather than repeated as warnings in the interface.

Only the low-resolution world has this didactic mode. In the high-resolution world mzLab behaves as a research aid: formula assignment, isotope-pattern comparison, spectral-library search (for instance with entropy-based similarity [@li2021]) and MSn trees return candidates with scores and the evidence behind them, in line with graded confidence reporting [@schymanski2014], never as truth.

The program is organised in four parts: *Data* (chromatograms, spectra, maps, integration with spreadsheet export, m/z calculator, periodic table, adducts, neutral losses, library comparison); *Drawing* (a chemical structure editor with formula and mass under each structure and mass difference over each reaction arrow); *Theory* (22 chapters on the transformation-product problem, chromatography, ionisation, analysers and the reading of EI and MS^2^ spectra, with interactive figures and a glossary); and *Practice* (exercises from EI spectrum to structure, oral-exam questions with spaced review, neutral losses, isotopes, exact formula, instrument choice). The interface is bilingual (Italian and English); Theory and Practice are in Italian. Colour choices follow a colour-blind-safe palette, reduced-motion settings are respected, and controls are keyboard accessible.

# Use in the course

mzLab is used in "experience 3" of the pollutant-analysis laboratory course at the Università di Torino. Students receive LC-MS files of a pollutant sampled at successive irradiation times, acquired on a SCIEX 3200 QTRAP triple quadrupole (ESI+, unit resolution) in full-scan, MS^2^ and MRM modes; they find candidate TPs from the time evolution of extracted ion chromatograms, support the candidates with MS^2^ spectra and neutral losses, draw proposed structures and pathways, and prepare the report in a spreadsheet. The theory and practice sections prepare the oral examination. The laboratory data are not distributed with the program; a few anonymised example files are included. This paper makes no claim about learning gains: no controlled data on the effect of mzLab on learning outcomes are yet available, and a dedicated pre/post study is planned separately.

# Architecture and privacy

The same Python code (file readers for mzML and vendor formats converted to it, chromatogram and peak routines, chemistry utilities) runs in two settings: a development server used for testing, and, in the published site, inside the browser through Pyodide, with a Web Worker and IndexedDB for files and session. The page is plain HTML and JavaScript with no build step and a strict content security policy. Files opened by students are processed locally and are never sent to a server; the site collects no visitor data. After the first visit (about 10–15 MB) the site works offline and can be installed as an app. An experimental WebAssembly engine written in Rust accelerates chromatogram extraction for large files. The code is open source under the MIT licence; third-party components keep their own licences.

# Limitations

mzLab has not yet been validated against vendor software on a wide range of compounds. Vendor files must be converted to mzML. It is tested most thoroughly in Chromium; Firefox and Safari have a reduced automatic test set. On smartphones only the Theory section is available.

# Acknowledgements

The software and this manuscript were developed with substantial assistance from AI coding agents (large language models), under the direction and review of the author, who is responsible for the content. Example data files were authorised by the instructor of the course. We thank the open-source projects mzLab builds on: Pyodide, NumPy, Ketcher and OpenChemLib.

# References
