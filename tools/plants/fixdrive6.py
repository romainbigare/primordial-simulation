s = open('drive.js').read()
old = """    const date = () => { const m = (st.textContent || '').match(/year (\\d+), day (\\d+)/); return m ? [+m[1], +m[2]] : [0, 0]; };
    const at = () => { const [y, d] = date(); return y + d / 365; };
    const wait = ms => new Promise(r => setTimeout(r, ms));
    // Pause the moment the date shown reaches `target`"""
new = """    const date = () => { const m = (st.textContent || '').match(/year (\\d+), day (\\d+)/); return m ? [+m[1], +m[2]] : [0, 0]; };
    // The year shown over the map (its fraction a month's), which follows the world more closely than the stats' date.
    const yearEl = document.getElementById('year');
    const at = () => parseFloat((yearEl.textContent || '').replace(/[^0-9.]/g, '')) || 0;
    const wait = ms => new Promise(r => setTimeout(r, ms));
    // Pause the moment the date shown reaches `target`"""
assert old in s
s = s.replace(old, new)
s = s.replace("    await stopAt(U - 45 / 365);", "    await stopAt(U - 1.5 / 12);")
old = "    const got = await runTo(+yearArg + lo / 365);"
assert old in s
s = s.replace(old, "    const got = await runTo(+yearArg + Math.round((lo + 13) / 30.4) / 12 - 0.001);")
open('drive.js', 'w').write(s)
