# XCMS (R): centWave -> obiwarp -> peak density -> fillChromPeaks -> `featureDefinitions` + `featureValues` as CSV (read by corri_xcms.py).
# Rscript corri_xcms.R PARAMETRI.json OUT.csv FILE.mzML [FILE.mzML ...]
suppressPackageStartupMessages({library(xcms); library(MSnbase); library(jsonlite)})
a <- commandArgs(trailingOnly = TRUE)
par <- fromJSON(a[1]); out <- a[2]; files <- a[-(1:2)]
register(if (length(files) > 1 && nzchar(Sys.getenv("BENCH_WORKERS"))) MulticoreParam(as.integer(Sys.getenv("BENCH_WORKERS"))) else SerialParam())
raw <- readMSData(files, mode = "onDisk", msLevel. = 1)
cw <- par$CentWaveParam
x <- findChromPeaks(raw, param = CentWaveParam(ppm = cw$ppm, peakwidth = c(cw$peakwidth_min, cw$peakwidth_max), snthresh = cw$snthresh,
                    prefilter = c(cw$prefilter_k, cw$prefilter_i), mzdiff = cw$mzdiff, noise = cw$noise, integrate = cw$integrate))
pd <- par$PeakDensityParam
if (length(files) > 1) {
  x <- adjustRtime(x, param = ObiwarpParam(binSize = par$ObiwarpParam$binSize))
  x <- groupChromPeaks(x, param = PeakDensityParam(sampleGroups = rep(1, length(files)), bw = pd$bw, binSize = pd$binSize, minFraction = pd$minFraction, minSamples = pd$minSamples))
} else {
  x <- groupChromPeaks(x, param = PeakDensityParam(sampleGroups = 1, bw = pd$bw, binSize = pd$binSize, minFraction = pd$minFraction, minSamples = pd$minSamples))
}
x <- fillChromPeaks(x, param = ChromPeakAreaParam())
d <- featureDefinitions(x); v <- featureValues(x, value = "into", method = "maxint")
colnames(v) <- basename(files)
res <- data.frame(mzmed = d$mzmed, rtmed = d$rtmed, npeaks = d$npeaks, v, check.names = FALSE)
write.csv(res, out, row.names = FALSE)
cat("xcms", as.character(packageVersion("xcms")), "\n")
