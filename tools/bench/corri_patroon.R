# patRoon (R, inside the Docker image): findFeatures -> groupFeatures -> filter -> optional screenSuspects + estimateIDConfidence -> CSV (read by corri_patroon.py).
# Rscript corri_patroon.R PARAMETRI.json OUT_DIR ANALISI.csv|- SOSPETTI.csv|- FILE.mzML [FILE.mzML ...]
# ANALISI.csv: analysis,group,blank (blank = the group name of the blanks subtracted); SOSPETTI.csv: name,mz,rt (rt in seconds, optional),SMILES (optional).
# patRoon is called as an external program: only its documented API and its tables are used here.
suppressPackageStartupMessages({library(patRoon); library(jsonlite)})
a <- commandArgs(trailingOnly = TRUE)
par <- fromJSON(a[1]); out <- a[2]; files <- a[-(1:4)]
stems <- sub("\\.mzML$", "", basename(files), ignore.case = TRUE)
if (a[3] != "-") {
  ana <- read.csv(a[3], stringsAsFactors = FALSE)
  ana <- ana[match(stems, ana$analysis), ]
  anaInfo <- data.frame(path = dirname(files), analysis = stems, group = ana$group, blank = ana$blank, stringsAsFactors = FALSE)
} else {
  anaInfo <- data.frame(path = dirname(files), analysis = stems, group = stems, blank = "", stringsAsFactors = FALSE)
}
ff <- par$findFeatures
if (ff$algorithm == "openms") {
  fl <- findFeatures(anaInfo, "openms", noiseThrInt = ff$noiseThrInt, chromSNR = ff$chromSNR, chromFWHM = ff$chromFWHM, mzPPM = ff$mzPPM, minFWHM = ff$minFWHM, maxFWHM = ff$maxFWHM)
} else {
  fl <- findFeatures(anaInfo, "xcms3", param = xcms::CentWaveParam(ppm = ff$ppm, peakwidth = c(ff$peakwidth_min, ff$peakwidth_max), snthresh = ff$snthresh,
                     prefilter = c(ff$prefilter_k, ff$prefilter_i), mzdiff = ff$mzdiff, noise = ff$noise))
}
gf <- par$groupFeatures
if (gf$algorithm == "openms") {
  fg <- groupFeatures(fl, "openms", rtalign = gf$rtalign, maxAlignRT = gf$maxAlignRT, maxAlignMZ = gf$maxAlignMZ, maxGroupRT = gf$maxGroupRT, maxGroupMZ = gf$maxGroupMZ)
} else {
  fg <- groupFeatures(fl, "xcms3", rtalign = gf$rtalign, groupParam = xcms::PeakDensityParam(sampleGroups = anaInfo$group, bw = gf$bw, binSize = gf$pdBinSize,
                      minFraction = gf$minFraction, minSamples = gf$minSamples), retAlignParam = xcms::ObiwarpParam(binSize = gf$binSize))
}
fi <- par$filter
fg <- filter(fg, absMinIntensity = fi$absMinIntensity, relMinReplicateAbundance = fi$relMinReplicateAbundance, maxReplicateIntRSD = fi$maxReplicateIntRSD,
             blankThreshold = if (a[3] != "-") fi$blankThreshold else NULL, removeBlanks = a[3] != "-")
write.csv(as.data.table(fg, average = FALSE, features = FALSE, areas = TRUE), file.path(out, "featureGroups.csv"), row.names = FALSE)
if (a[4] != "-") {
  su <- par$screenSuspects
  fg <- screenSuspects(fg, read.csv(a[4], stringsAsFactors = FALSE), rtWindow = su$rtWindow, mzWindow = su$mzWindow, adduct = su$adduct, onlyHits = TRUE)
  # no MS/MS peak lists here: the confidence level comes from the suspect list and the data given to patRoon
  fg <- annotateSuspects(fg)
  si <- as.data.frame(screenInfo(fg))
  if (!"estIDLevel" %in% names(si)) si$estIDLevel <- NA
  if (!"score" %in% names(si)) si$score <- NA
  write.csv(si[, c("group", "name", "estIDLevel", "score")], file.path(out, "suspects.csv"), row.names = FALSE)
}
cat("patRoon", as.character(packageVersion("patRoon")), "\n")
