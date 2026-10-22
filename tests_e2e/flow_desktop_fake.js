// Test fake of the native routes flow/live/* of the desktop app (e2e_flow.py): a node event per step, a fixed time per phase.
() => { window.MZLAB_DESKTOP = true; window.__runs = 0; window.__sent = []; window.__t0 = 0; window.__ops = []; const real = window.fetch.bind(window);
          const node = (seq, id, state, ms) => ({ format: 'flowlive/1', seq, rev: 0, kind: 'node', instance: id, state, frac: null, parte: state === 'done' ? 1 : 0, di: 1, ms: ms || null, key: null, blocked_by: null, error: null });
          window.fetch = async (u, o) => { if (!String(u).includes('/rpc')) return real(u, o);
            const b = JSON.parse(o.body), op = b.op, ok = (v) => new Response(typeof v === 'string' ? v : JSON.stringify(v), { status: 200 });
            window.__ops.push(op);
            if (op === 'flow/read') return ok('a,b\n1,2\n');
            if (op === 'flow/live/start') { const plan = JSON.parse(b.plan); window.__runs++; window.__sent.push(plan.inputs.map(i => i.name).join(','));
              if (window.__missing && (window.__missing === '*' || plan.inputs.some(i => i.name === window.__missing))) return new Response(JSON.stringify({ error_key: 'err.flow.input.missing', params: { input: window.__missing === '*' ? 'a.mzML' : window.__missing } }), { status: 400 });
              window.__plan = plan; window.__t0 = Date.now(); return ok({}); }
            const steps = (window.__plan || { steps: [] }).steps, late = Date.now() - window.__t0 > 2000 || window.__quick;
            if (op === 'flow/live/events') { const n = steps.length, ev = late ? steps.map((x, i) => node(n + i + 1, x.id, 'done', 1200)) : steps.map((x, i) => node(i + 1, x.id, i === 0 ? 'done' : 'ready', i === 0 ? 900 : 0));
              return ok(ev.filter(e => e.seq > b.since)); }
            if (op === 'flow/live/nodes') return ok(steps.map((x, i) => ({ instance: x.id, state: late || i === 0 ? 'done' : 'ready' })));
            if (op === 'flow/live/finish') { const activity = {}; steps.forEach((x, i) => { activity[x.id] = { step: x.step, ms: 1200, used: [], generated: ['k'], ...(i === steps.length - 1 ? { cached: true } : {}) }; });
              return ok({ prov: JSON.stringify({ schema: 'prov/1', entity: { k: { type: 'table', label: 't.csv' } }, activity }), dir: '/tmp/mzlab-flow/out' }); }
            return ok(op === 'flow/live/update' ? [] : {}); }; }
