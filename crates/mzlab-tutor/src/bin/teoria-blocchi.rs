//! `teoria-blocchi DIR OUT [--controlla]`: cuts the Teoria pages of DIR into blocks and writes OUT (`teoria-blocchi/1`);
//! with `--controlla` it only compares the result with OUT and exits 1 when they differ.
use std::{fs, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|a| a == "--controlla");
    let pos: Vec<&String> = args.iter().filter(|a| *a != "--controlla").collect();
    let [dir, out] = pos[..] else {
        eprintln!("uso: teoria-blocchi DIR OUT [--controlla]");
        return ExitCode::from(2);
    };
    match run(Path::new(dir), Path::new(out), check) {
        Ok(same) => {
            if same {
                ExitCode::SUCCESS
            } else {
                eprintln!("{out} non corrisponde alle pagine di {dir}");
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("errore: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(dir: &Path, out: &Path, check: bool) -> Result<bool, String> {
    let mut pages = Vec::new();
    for e in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let p = e.map_err(|e| e.to_string())?.path();
        if p.extension().is_some_and(|x| x == "html") {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            pages.push((
                name,
                fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?,
            ));
        }
    }
    let text = mzlab_tutor::blocchi::to_text(&mzlab_tutor::blocchi::build(&pages)?);
    if check {
        return Ok(fs::read_to_string(out).is_ok_and(|old| old == text));
    }
    fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(true)
}
