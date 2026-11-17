"""More pairs of `unindexed` (tools/wiff_riferimento.py): negative baseline noise, and table rows queued without a section."""


def unindexed_more(out, Writer, method, mine, a):
    def donor():
        w = Writer()
        w.sample(1, [("mrm", v, 1000.0 * k, sum(v)) for k, v in enumerate(mine)], method)
        return w

    # `nonindicizzato_guasto`: a NaN count is not what the layout says (left out)
    w = donor()
    w.unindexed(2, [[1.0, 2.0], [float("nan"), 4.0]])
    w.save(out, "nonindicizzato_guasto")
    # `nonindicizzato_negativo`: a dense block with negative baseline noise is a valid block (recovered)
    w = donor()
    w.unindexed(2, [[1.0, -2.5], [-3.0, 4.0]])
    w.save(out, "nonindicizzato_negativo")
    # `nonindicizzato_in_coda`: table rows with no section (offset 0, a batch queued before the acquisition) must not hide the section being acquired
    w = donor()
    w.table += [(2, 0), (3, 0)]
    w.unindexed(2, a)
    w.save(out, "nonindicizzato_in_coda")
