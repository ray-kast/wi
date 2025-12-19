use std::{
    collections::BTreeMap,
    env,
    fmt::{self, Write},
    fs, mem,
    path::Path,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Weight {
    None,
    Light,
    Heavy,
}

impl fmt::Display for Weight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::None => "S0",
            Self::Light => "S1",
            Self::Heavy => "SX",
        })
    }
}

#[expect(clippy::too_many_lines, reason = "I just don't care.......")]
fn main() {
    let mut arms = [([Weight::None; 4], None::<char>)]
        .into_iter()
        .collect::<BTreeMap<_, _>>();

    let mut buf = String::new();
    for char in '\u{2500}'..'\u{2580}' {
        buf.extend(unicode_names2::name(char).unwrap());

        'char: {
            let mut weights = [Weight::None; 4];

            let mut id = buf.strip_prefix("BOX DRAWINGS").unwrap().trim_start();
            let mut last_dir: Option<[bool; _]> = None;
            let mut last_weight = None;
            let mut strip_and = false;

            while let Some((l, r)) = id
                .split_once(' ')
                .or_else(|| (!id.is_empty()).then_some((id, "")))
            {
                id = if matches!(l, "ARC" | "DIAGONAL" | "SINGLE" | "DOUBLE")
                    || r.starts_with("DASH")
                {
                    break 'char;
                } else if matches!(l, "AND") {
                    r
                } else if let Some(dir) = match l {
                    "LEFT" => Some([true, false, false, false]),
                    "UP" => Some([false, true, false, false]),
                    "RIGHT" => Some([false, false, true, false]),
                    "DOWN" => Some([false, false, false, true]),
                    "HORIZONTAL" => Some([true, false, true, false]),
                    "VERTICAL" => Some([false, true, false, true]),
                    _ => None,
                } {
                    if let Some(weight) = last_weight.take() {
                        for w in weights
                            .iter_mut()
                            .zip(dir)
                            .filter_map(|(w, d)| d.then_some(w))
                        {
                            assert!(matches!(mem::replace(w, weight), Weight::None));
                        }

                        if mem::take(&mut strip_and) {
                            last_weight = Some(weight);
                            r.strip_prefix("AND ").unwrap_or(r)
                        } else {
                            r
                        }
                    } else if let Some(prev) = last_dir.as_mut() {
                        for (p, d) in prev.iter_mut().zip(dir) {
                            *p |= d;
                        }

                        r
                    } else {
                        last_dir = Some(dir);
                        r
                    }
                } else if let Some(weight) = match l {
                    "LIGHT" | "SINGLE" => Some(Weight::Light),
                    "HEAVY" | "DOUBLE" => Some(Weight::Heavy),
                    _ => None,
                } {
                    if let Some(dir) = last_dir.take() {
                        for w in weights
                            .iter_mut()
                            .zip(dir)
                            .filter_map(|(w, d)| d.then_some(w))
                        {
                            assert!(matches!(mem::replace(w, weight), Weight::None));
                        }
                    } else {
                        last_weight = Some(weight);
                        strip_and = true;
                    }

                    r
                } else {
                    panic!("{l} ^ {r}");
                }
            }

            assert!(last_dir.is_none());

            arms.insert(weights, Some(char));
        }

        buf.clear();
    }

    let mut contents: String = arms.into_iter().fold(
        String::from(
            "pub(super) const fn cell_char(left: CellState, top: CellState, right: CellState, \
             bottom: CellState) -> Option<char> { match (left, top, right, bottom) { ",
        ),
        |mut s, ([l, t, r, b], c)| {
            write!(s, "({l}, {t}, {r}, {b}) => ").unwrap();
            if let Some(c) = c {
                write!(s, "Some('{c}')").unwrap();
            } else {
                s.push_str("None");
            }
            s.push(',');
            s
        },
    );
    contents.push_str(" } }");

    fs::write(
        Path::new(&env::var_os("OUT_DIR").unwrap()).join("edge_cell_char.rs"),
        contents,
    )
    .unwrap();

    println!("cargo::rerun-if-changed=build.rs");
}
