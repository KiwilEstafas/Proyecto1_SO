use std::env;
use threadcity::model::Grid; 

fn main() {
    let args: Vec<String> = env::args().collect();
    let (rows, cols) = if args.len() >= 3 {
        let r = args[1].parse::<u32>().unwrap_or(5);
        let c = args[2].parse::<u32>().unwrap_or(5);
        (r, c)
    } else {
        (5, 5)
    };

    let grid = Grid::new(rows, cols);
    println!("grid.size() = {} ({} x {})", grid.size(), grid.rows, grid.cols);

    // listar al menos 25 celdas (o menos si el grid es menor)
    let mut printed: u32 = 0;
    let target: u32 = 25;

    for r in 0..grid.rows {
        for c in 0..grid.cols {
            println!("cell[{}] = ({},{})", printed, r, c);
            printed += 1;
            if printed >= target {
                break;
            }
        }
        if printed >= target {
            break;
        }
    }

    // validacion simple del criterio
    if grid.size() < 25 {
        eprintln!(
            "ADVERTENCIA: el grid actual tiene {} celdas (< 25). Usa dimensiones mayores (p.ej. 5x5).",
            grid.size()
        );
        std::process::exit(1);
    }
}
